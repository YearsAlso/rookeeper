//! snapshot.rs
//!
//! 快照模块，实现状态快照的创建、保存和恢复功能。

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Result as IoResult, Write};
use std::path::{Path, PathBuf};

use crate::StorageLayout;

/// 快照结构
///
/// 包含某一时刻的完整状态数据
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// 快照代际号
    pub generation: u64,
    /// 键值状态
    pub state: HashMap<String, Vec<u8>>,
    /// 元数据
    pub metadata: HashMap<String, String>,
}

impl Snapshot {
    /// 创建新的快照
    pub fn new(generation: u64) -> Self {
        Self {
            generation,
            state: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    /// 从现有状态创建快照
    pub fn from_state(generation: u64, state: HashMap<String, Vec<u8>>) -> Self {
        Self {
            generation,
            state,
            metadata: HashMap::new(),
        }
    }

    /// 设置元数据
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// 获取元数据
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// 编码为字节数组
    ///
    /// 格式：
    /// - generation(8)
    /// - state_len(4) + state entries (key_len(4) + key + value_len(4) + value)
    /// - metadata_len(4) + metadata entries (key_len(4) + key + value_len(4) + value)
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // generation
        buf.extend_from_slice(&self.generation.to_le_bytes());

        // state
        buf.extend_from_slice(&(self.state.len() as u32).to_le_bytes());
        for (key, value) in &self.state {
            let key_bytes = key.as_bytes();
            buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(key_bytes);
            buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
            buf.extend_from_slice(value);
        }

        // metadata
        buf.extend_from_slice(&(self.metadata.len() as u32).to_le_bytes());
        for (key, value) in &self.metadata {
            let key_bytes = key.as_bytes();
            let value_bytes = value.as_bytes();
            buf.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(key_bytes);
            buf.extend_from_slice(&(value_bytes.len() as u32).to_le_bytes());
            buf.extend_from_slice(value_bytes);
        }

        buf
    }

    /// 从字节数组解码
    pub fn decode(buf: &[u8]) -> Result<Self, SnapshotError> {
        let mut offset = 0;

        // generation
        let generation = u64::from_le_bytes(
            buf[offset..offset + 8]
                .try_into()
                .map_err(|_| SnapshotError::DecodeError("generation".to_string()))?,
        );
        offset += 8;

        // state
        let state_len = u32::from_le_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|_| SnapshotError::DecodeError("state_len".to_string()))?,
        ) as usize;
        offset += 4;

        let mut state = HashMap::new();
        for _ in 0..state_len {
            let key_len = u32::from_le_bytes(
                buf[offset..offset + 4]
                    .try_into()
                    .map_err(|_| SnapshotError::DecodeError("state_key_len".to_string()))?,
            ) as usize;
            offset += 4;

            let key = String::from_utf8(buf[offset..offset + key_len].to_vec())
                .map_err(|_| SnapshotError::DecodeError("state_key".to_string()))?;
            offset += key_len;

            let value_len = u32::from_le_bytes(
                buf[offset..offset + 4]
                    .try_into()
                    .map_err(|_| SnapshotError::DecodeError("state_value_len".to_string()))?,
            ) as usize;
            offset += 4;

            let value = buf[offset..offset + value_len].to_vec();
            offset += value_len;

            state.insert(key, value);
        }

        // metadata
        let metadata_len = u32::from_le_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|_| SnapshotError::DecodeError("metadata_len".to_string()))?,
        ) as usize;
        offset += 4;

        let mut metadata = HashMap::new();
        for _ in 0..metadata_len {
            let key_len = u32::from_le_bytes(
                buf[offset..offset + 4]
                    .try_into()
                    .map_err(|_| SnapshotError::DecodeError("metadata_key_len".to_string()))?,
            ) as usize;
            offset += 4;

            let key = String::from_utf8(buf[offset..offset + key_len].to_vec())
                .map_err(|_| SnapshotError::DecodeError("metadata_key".to_string()))?;
            offset += key_len;

            let value_len = u32::from_le_bytes(
                buf[offset..offset + 4]
                    .try_into()
                    .map_err(|_| SnapshotError::DecodeError("metadata_value_len".to_string()))?,
            ) as usize;
            offset += 4;

            let value = String::from_utf8(buf[offset..offset + value_len].to_vec())
                .map_err(|_| SnapshotError::DecodeError("metadata_value".to_string()))?;
            offset += value_len;

            metadata.insert(key, value);
        }

        Ok(Self {
            generation,
            state,
            metadata,
        })
    }

    /// 保存到文件
    pub fn save_to(&self, path: impl AsRef<Path>) -> IoResult<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;

        let encoded = self.encode();
        file.write_all(&encoded)?;
        file.sync_all()
    }

    /// 从文件加载
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, SnapshotError> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        Self::decode(&buffer)
    }
}

/// 快照错误类型
#[derive(Debug)]
pub enum SnapshotError {
    /// IO 错误
    IoError(std::io::Error),
    /// 解码错误
    DecodeError(String),
}

impl From<std::io::Error> for SnapshotError {
    fn from(err: std::io::Error) -> Self {
        SnapshotError::IoError(err)
    }
}

/// 快照管理器
///
/// 负责创建、保存和加载快照
#[derive(Debug)]
pub struct SnapshotManager {
    /// 存储布局
    layout: StorageLayout,
    /// 当前代际号
    current_generation: u64,
}

impl SnapshotManager {
    /// 创建新的快照管理器
    pub fn new(layout: StorageLayout) -> Self {
        Self {
            layout,
            current_generation: 0,
        }
    }

    /// 获取存储布局
    pub fn layout(&self) -> &StorageLayout {
        &self.layout
    }

    /// 获取当前代际号
    pub fn current_generation(&self) -> u64 {
        self.current_generation
    }

    /// 设置当前代际号
    pub fn set_current_generation(&mut self, generation: u64) {
        self.current_generation = generation;
    }

    /// 创建新快照
    pub fn create_snapshot(
        &self,
        state: HashMap<String, Vec<u8>>,
    ) -> Result<Snapshot, SnapshotError> {
        let generation = self.current_generation + 1;
        let mut snapshot = Snapshot::from_state(generation, state);
        snapshot.set_metadata("created_at", chrono_timestamp());
        Ok(snapshot)
    }

    /// 保存快照到文件
    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<PathBuf, SnapshotError> {
        let path = self.layout.snapshot_path(snapshot.generation);
        snapshot.save_to(&path)?;
        Ok(path)
    }

    /// 加载最新的快照
    pub fn load_latest(&self) -> Result<Option<Snapshot>, SnapshotError> {
        // 扫描快照目录查找最新的快照文件
        let snapshot_dir = &self.layout.snapshot_dir;

        if !snapshot_dir.exists() {
            return Ok(None);
        }

        let mut entries = std::fs::read_dir(snapshot_dir)?
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
        let snapshot = Snapshot::load_from(&latest_path)?;

        Ok(Some(snapshot))
    }

    /// 获取恢复游标
    ///
    /// 根据最新快照和 WAL 状态生成恢复游标
    pub fn get_recovery_cursor(&self, wal_segment_id: u64, wal_offset: u64) -> RecoveryCursor {
        use crate::state::RecoveryCursor;

        let snapshot_generation = self.current_generation;
        let mut cursor = RecoveryCursor {
            version: 1,
            snapshot_generation,
            wal_segment_id,
            wal_offset,
            checksum: 0,
        };
        cursor.checksum = cursor.compute_checksum();

        cursor
    }
}

/// 获取当前时间戳（简化的 ISO 格式）
fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

// 重新导出 RecoveryCursor 以便在模块间使用
pub use crate::state::RecoveryCursor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_encode_decode() {
        let mut state = HashMap::new();
        state.insert("key1".to_string(), vec![1, 2, 3]);
        state.insert("key2".to_string(), vec![4, 5, 6]);

        let snapshot = Snapshot::from_state(1, state);
        let encoded = snapshot.encode();
        let decoded = Snapshot::decode(&encoded).unwrap();

        assert_eq!(decoded.generation, snapshot.generation);
        assert_eq!(decoded.state.len(), 2);
        assert_eq!(decoded.state.get("key1"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_snapshot_metadata() {
        let mut snapshot = Snapshot::new(1);
        snapshot.set_metadata("author", "test");
        assert_eq!(snapshot.get_metadata("author"), Some(&"test".to_string()));
    }
}
