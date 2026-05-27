//! rookeeper-storage
//! 
//! 存储布局约定和数据文件命名规范。
//! 
//! 设计原则：
//! - 所有存储路径通过 StorageLayout 统一管理，避免硬编码路径散落各处
//! - WAL 和快照文件名使用固定格式便于恢复时扫描
//! - CRC32 校验确保数据完整性检测

use std::path::{Path, PathBuf};

/// 默认的分布式锁文件名，确保单实例运行
pub const DEFAULT_LOCK_FILE: &str = "rookeeper.lock";

/// 存储布局结构，定义了数据目录的组织方式
/// 
/// 布局设计：
/// - `wal/` - 预写日志，顺序写入用于崩溃恢复
/// - `snapshot/` - 定期快照，用于加速启动和缩小 WAL
/// - `state/` - 集群元数据，如成员信息和配置版本
/// - `rookeeper.lock` - 分布式锁文件，防止多实例同时运行
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLayout {
    /// 数据根目录
    pub root: PathBuf,
    /// WAL 目录
    pub wal_dir: PathBuf,
    /// 快照目录
    pub snapshot_dir: PathBuf,
    /// 集群状态目录
    pub state_dir: PathBuf,
    /// 分布式锁文件路径
    pub lock_file: PathBuf,
}

impl StorageLayout {
    /// 从根目录构造完整的存储布局
    /// 
    /// 所有子目录和文件都相对于根目录计算
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            wal_dir: root.join("wal"),
            snapshot_dir: root.join("snapshot"),
            state_dir: root.join("state"),
            lock_file: root.join(DEFAULT_LOCK_FILE),
            root,
        }
    }

    /// 生成指定 WAL 段的文件路径
    /// 
    /// 使用 16 位零填充格式确保文件列表按序号排序
    /// 格式：`wal-{segment_id:016}.log`
    pub fn wal_segment_path(&self, segment_id: u64) -> PathBuf {
        self.wal_dir.join(format!("wal-{segment_id:016}.log"))
    }

    /// 生成指定代际的快照文件路径
    /// 
    /// 使用 16 位零填充格式确保文件列表按代际排序
    /// 格式：`snapshot-{generation:016}.bin`
    pub fn snapshot_path(&self, generation: u64) -> PathBuf {
        self.snapshot_dir
            .join(format!("snapshot-{generation:016}.bin"))
    }
}

/// 计算数据的 CRC32 校验和
/// 
/// 用于检测传输或存储过程中的数据损坏
pub fn checksum(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

#[cfg(test)]
mod tests {
    use super::{checksum, StorageLayout};

    #[test]
    fn builds_expected_layout() {
        let layout = StorageLayout::from_root("data");
        assert!(layout
            .wal_segment_path(7)
            .ends_with("wal-0000000000000007.log"));
        assert!(layout
            .snapshot_path(3)
            .ends_with("snapshot-0000000000000003.bin"));
    }

    #[test]
    fn checksum_is_stable() {
        // 校验和稳定确保升级后旧数据仍可验证
        assert_eq!(checksum(b"rookeeper"), 2_164_299_473);
    }
}