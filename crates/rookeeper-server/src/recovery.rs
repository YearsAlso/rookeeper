//! recovery.rs
//!
//! 恢复管理器，实现从快照和 WAL 重放恢复状态机的逻辑。
//!
//! 设计原则：
//! - 恢复顺序严格遵循 storage-layout.md 的规定
//! - 状态机必须是确定性的，不依赖随机数或墙上时钟

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

use rookeeper_storage::prelude::{RecoveryCursor, WalEntry, WalReader, WalWriter};
use rookeeper_storage::StorageLayout;

use crate::tree_kv::{TreeKv, TreeKvSnapshot};

/// 恢复错误类型
#[derive(Debug)]
pub enum RecoveryError {
    /// 获取锁失败
    LockFailed,
    /// 快照加载失败
    SnapshotFailed(String),
    /// WAL 重放失败
    WalReplayFailed(String),
    /// 恢复游标损坏
    CursorCorrupted,
    /// IO 错误
    IoError(std::io::Error),
}

impl From<std::io::Error> for RecoveryError {
    fn from(err: std::io::Error) -> Self {
        RecoveryError::IoError(err)
    }
}

/// 恢复管理器
///
/// 负责管理启动恢复流程：
/// 1. 获取分布式锁，确保单实例恢复
/// 2. 读取最新快照
/// 3. 从快照之后的 WAL 段继续顺序重放
/// 4. 状态机恢复完成后开放服务请求
#[derive(Debug)]
pub struct RecoveryManager {
    /// 存储布局
    layout: StorageLayout,
    /// 是否已获取锁
    lock_acquired: bool,
}

impl RecoveryManager {
    /// 创建新的恢复管理器
    pub fn new(layout: StorageLayout) -> Self {
        Self {
            layout,
            lock_acquired: false,
        }
    }

    /// 获取分布式锁
    ///
    /// 确保同一时刻只有一个 rookeeper 实例运行
    /// 如果锁文件已存在且被其他实例持有，则返回错误
    pub fn acquire_lock(&mut self) -> Result<(), RecoveryError> {
        let lock_path = &self.layout.lock_file;

        // 尝试创建锁文件
        let mut file = match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(lock_path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(RecoveryError::LockFailed);
            }
            Err(e) => return Err(RecoveryError::IoError(e)),
        };

        // 写入当前进程信息（用于调试）
        let pid = std::process::id().to_string();
        file.write_all(pid.as_bytes())?;
        file.sync_all()?;

        self.lock_acquired = true;
        Ok(())
    }

    /// 释放分布式锁
    pub fn release_lock(&mut self) -> Result<(), RecoveryError> {
        if !self.lock_acquired {
            return Ok(());
        }

        std::fs::remove_file(&self.layout.lock_file)?;
        self.lock_acquired = false;
        Ok(())
    }

    /// 加载最新的快照
    ///
    /// # Returns
    /// - `Ok(Some(snapshot))` - 找到并加载了快照
    /// - `Ok(None)` - 没有快照可用
    /// - `Err(RecoveryError)` - 加载失败
    pub fn load_latest_snapshot(&self) -> Result<Option<TreeKvSnapshot>, RecoveryError> {
        // 扫描快照目录查找最新的快照文件
        let snapshot_dir = &self.layout.snapshot_dir;

        if !snapshot_dir.exists() {
            return Ok(None);
        }

        let mut entries = std::fs::read_dir(snapshot_dir)
            .map_err(|e| {
                RecoveryError::SnapshotFailed(format!("failed to read snapshot dir: {}", e))
            })?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("snapshot-") && n.ends_with(".bin"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        if entries.is_empty() {
            return Ok(None);
        }

        // 按文件名排序获取最新的
        entries.sort_by_key(|e| e.path());

        let latest_path = entries.last().unwrap().path();

        // 读取快照文件
        let mut file = File::open(&latest_path).map_err(|e| {
            RecoveryError::SnapshotFailed(format!("failed to open snapshot: {}", e))
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|e| {
            RecoveryError::SnapshotFailed(format!("failed to read snapshot: {}", e))
        })?;

        // 使用 bincode 反序列化
        let snapshot: TreeKvSnapshot = bincode::deserialize(&buffer).map_err(|e| {
            RecoveryError::SnapshotFailed(format!("failed to deserialize snapshot: {}", e))
        })?;

        Ok(Some(snapshot))
    }

    /// 保存快照
    ///
    /// # Arguments
    /// * `snapshot` - 要保存的快照
    /// * `generation` - 快照代际号
    pub fn save_snapshot(
        &self,
        snapshot: &TreeKvSnapshot,
        generation: u64,
    ) -> Result<(), RecoveryError> {
        let path = self.layout.snapshot_path(generation);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 序列化并写入
        let data = bincode::serialize(snapshot).map_err(|e| {
            RecoveryError::SnapshotFailed(format!("failed to serialize snapshot: {}", e))
        })?;

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .map_err(|e| {
                RecoveryError::SnapshotFailed(format!("failed to open snapshot file: {}", e))
            })?;

        file.write_all(&data)?;
        file.sync_all()?;

        Ok(())
    }

    /// 加载恢复游标
    ///
    /// 从 cluster.meta 文件加载恢复进度
    pub fn load_recovery_cursor(&self) -> Result<RecoveryCursor, RecoveryError> {
        let path = self.layout.state_path("cluster");

        if !path.exists() {
            return Ok(RecoveryCursor::default());
        }

        let cursor = RecoveryCursor::load_from(&path)?;
        Ok(cursor)
    }

    /// 保存恢复游标
    pub fn save_recovery_cursor(&self, cursor: &RecoveryCursor) -> Result<(), RecoveryError> {
        let path = self.layout.state_path("cluster");

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        cursor.save_to(&path)?;
        Ok(())
    }

    /// 重放 WAL 条目到状态机
    ///
    /// # Arguments
    /// * `tree` - 树形 KV 状态机
    /// * `start_segment` - 起始段 ID
    /// * `start_offset` - 起始偏移量
    ///
    /// # Returns
    /// - `Ok((last_segment, last_offset))` - 重放完成的最后位置
    pub fn replay_wal(
        &self,
        tree: &mut TreeKv,
        start_segment: u64,
        start_offset: u64,
    ) -> Result<(u64, u64), RecoveryError> {
        // 由于 path_cb 需要 'static 生命周期，我们使用 Arc<StorageLayout> 来共享 layout
        use std::sync::Arc;
        let layout = Arc::new(self.layout.clone());
        let layout_cb = Arc::clone(&layout);
        let path_cb = move |id: u64| layout_cb.wal_segment_path(id);

        // 创建 WAL 读取器
        let mut reader = WalReader::new(start_segment, path_cb).map_err(|e| {
            RecoveryError::WalReplayFailed(format!("failed to create WAL reader: {}", e))
        })?;

        // 如果有起始偏移量，需要跳过前面的条目
        // 注意：这里简化处理，实际实现需要更复杂的偏移量管理
        let mut offset = 0u64;
        while offset < start_offset {
            match reader.read_entry() {
                Ok(Some(_)) => {
                    // 计算每条条目的长度并跳过
                    offset += 1; // 简化：每条算一个偏移量
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(RecoveryError::WalReplayFailed(format!(
                        "failed to read WAL: {:?}",
                        e
                    )))
                }
            }
        }

        // 重放剩余的条目
        let mut last_segment = reader.segment_id();
        loop {
            match reader.read_entry() {
                Ok(Some(entry)) => {
                    // 将 WAL 条目应用到状态机
                    self.apply_entry(tree, &entry);
                    last_segment = reader.segment_id();
                }
                Ok(None) => {
                    // 当前段结束，尝试切换到下一段
                    if reader.advance_segment()? {
                        continue;
                    }
                    break;
                }
                Err(e) => {
                    return Err(RecoveryError::WalReplayFailed(format!(
                        "failed to read WAL entry: {:?}",
                        e
                    )))
                }
            }
        }

        Ok((last_segment, reader.offset()))
    }

    /// 将 WAL 条目应用到状态机
    fn apply_entry(&self, tree: &mut TreeKv, entry: &WalEntry) {
        use bytes::Bytes;
        use rookeeper_protocol::model::NodePath;
        use rookeeper_storage::prelude::OpType;

        let path = match NodePath::parse(&entry.path) {
            Ok(p) => p,
            Err(_) => return, // 跳过无效路径
        };

        match entry.op_type {
            OpType::Create => {
                let value = entry.value.clone().unwrap_or_default();
                let _ = tree.create(&path, Bytes::from(value), "system", None);
            }
            OpType::Set => {
                let value = entry.value.clone().unwrap_or_default();
                let _ = tree.set(&path, Bytes::from(value), None, "system", None);
            }
            OpType::Delete => {
                let _ = tree.delete(&path, "system");
            }
            _ => {
                // 其他操作类型暂不支持
            }
        }
    }

    /// 创建 WAL 写入器
    pub fn create_wal_writer(&self, segment_id: u64) -> Result<WalWriter, RecoveryError> {
        use std::sync::Arc;
        let layout = Arc::new(self.layout.clone());
        let layout_cb = Arc::clone(&layout);
        let path_cb = Box::new(move |id: u64| layout_cb.wal_segment_path(id));

        WalWriter::with_path_cb(segment_id, 64 * 1024 * 1024, path_cb)
            .map_err(RecoveryError::IoError)
    }

    /// 执行完整的恢复流程
    ///
    /// # Returns
    /// - `Ok(tree)` - 恢复完成的状态机
    pub fn recover(&mut self) -> Result<TreeKv, RecoveryError> {
        // 1. 获取分布式锁
        self.acquire_lock()?;

        // 2. 加载恢复游标
        let cursor = self.load_recovery_cursor()?;

        // 3. 创建空状态机
        let mut tree = TreeKv::new();

        // 4. 如果有快照，先从快照恢复
        if let Some(snapshot) = self.load_latest_snapshot()? {
            tree.restore_from_snapshot(&snapshot);
        }

        // 5. 从快照之后的 WAL 段继续顺序重放
        let (last_segment, last_offset) =
            self.replay_wal(&mut tree, cursor.wal_segment_id, cursor.wal_offset)?;

        // 6. 更新并保存恢复游标
        let mut new_cursor = RecoveryCursor::default();
        new_cursor.wal_segment_id = last_segment;
        new_cursor.wal_offset = last_offset;
        new_cursor.checksum = new_cursor.compute_checksum();

        self.save_recovery_cursor(&new_cursor)?;

        Ok(tree)
    }
}

/// 重放上下文，包含恢复所需的全部信息
#[derive(Debug)]
#[allow(dead_code)]
pub struct ReplayContext {
    /// 当前状态机
    pub tree: TreeKv,
    /// 当前 WAL 段 ID
    pub wal_segment_id: u64,
    /// 当前 WAL 偏移量
    pub wal_offset: u64,
}

impl ReplayContext {
    /// 创建新的重放上下文
    #[allow(dead_code)]
    pub fn new(tree: TreeKv) -> Self {
        Self {
            tree,
            wal_segment_id: 0,
            wal_offset: 0,
        }
    }

    /// 更新重放位置
    #[allow(dead_code)]
    pub fn update_position(&mut self, segment_id: u64, offset: u64) {
        self.wal_segment_id = segment_id;
        self.wal_offset = offset;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_cursor_default() {
        let cursor = RecoveryCursor::default();
        assert_eq!(cursor.version, 1);
        assert_eq!(cursor.snapshot_generation, 0);
        assert_eq!(cursor.wal_segment_id, 0);
        assert_eq!(cursor.wal_offset, 0);
    }
}
