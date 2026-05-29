//! state.rs
//!
//! 状态管理模块，包含恢复游标（RecoveryCursor）的定义和实现。

use std::fs::{File, OpenOptions};
use std::io::{Read, Result as IoResult, Write};
use std::path::Path;

/// RecoveryCursor 用于崩溃恢复时记录进度
///
/// 记录了恢复点所需的快照代际、WAL 位置等信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCursor {
    /// 版本号
    pub version: u32,
    /// 快照代际
    pub snapshot_generation: u64,
    /// WAL 段 ID
    pub wal_segment_id: u64,
    /// WAL 偏移量
    pub wal_offset: u64,
    /// 校验和
    pub checksum: u32,
}

impl Default for RecoveryCursor {
    fn default() -> Self {
        Self {
            version: 1,
            snapshot_generation: 0,
            wal_segment_id: 0,
            wal_offset: 0,
            checksum: 0,
        }
    }
}

impl RecoveryCursor {
    /// 从文件加载恢复游标
    ///
    /// 文件格式：version(4) + snapshot_generation(8) + wal_segment_id(8) + wal_offset(8) + checksum(4) = 32 bytes
    pub fn load_from(path: impl AsRef<Path>) -> IoResult<Self> {
        let mut file = File::open(path)?;
        let mut buffer = vec![0u8; 32];
        file.read_exact(&mut buffer)?;

        let version = u32::from_le_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]);
        let snapshot_generation = u64::from_le_bytes([
            buffer[4], buffer[5], buffer[6], buffer[7], buffer[8], buffer[9], buffer[10],
            buffer[11],
        ]);
        let wal_segment_id = u64::from_le_bytes([
            buffer[12], buffer[13], buffer[14], buffer[15], buffer[16], buffer[17], buffer[18],
            buffer[19],
        ]);
        let wal_offset = u64::from_le_bytes([
            buffer[20], buffer[21], buffer[22], buffer[23], buffer[24], buffer[25], buffer[26],
            buffer[27],
        ]);
        let checksum = u32::from_le_bytes([buffer[28], buffer[29], buffer[30], buffer[31]]);

        Ok(Self {
            version,
            snapshot_generation,
            wal_segment_id,
            wal_offset,
            checksum,
        })
    }

    /// 将恢复游标保存到文件
    ///
    /// 文件格式：version(4) + snapshot_generation(8) + wal_segment_id(8) + wal_offset(8) + checksum(4) = 32 bytes
    pub fn save_to(&self, path: impl AsRef<Path>) -> IoResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let mut buffer = [0u8; 32];
        buffer[0..4].copy_from_slice(&self.version.to_le_bytes());
        buffer[4..12].copy_from_slice(&self.snapshot_generation.to_le_bytes());
        buffer[12..20].copy_from_slice(&self.wal_segment_id.to_le_bytes());
        buffer[20..28].copy_from_slice(&self.wal_offset.to_le_bytes());
        buffer[28..32].copy_from_slice(&self.checksum.to_le_bytes());

        file.write_all(&buffer)?;
        file.sync_all()
    }

    /// 计算当前状态的校验和
    pub fn compute_checksum(&self) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&self.version.to_le_bytes());
        hasher.update(&self.snapshot_generation.to_le_bytes());
        hasher.update(&self.wal_segment_id.to_le_bytes());
        hasher.update(&self.wal_offset.to_le_bytes());
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_recovery_cursor_save_load() {
        let temp_dir = temp_dir();
        let path = temp_dir.join("test_cursor.bin");

        let cursor = RecoveryCursor {
            version: 1,
            snapshot_generation: 100,
            wal_segment_id: 5,
            wal_offset: 12345,
            checksum: 0,
        };

        cursor.save_to(&path).unwrap();
        let loaded = RecoveryCursor::load_from(&path).unwrap();

        assert_eq!(cursor.version, loaded.version);
        assert_eq!(cursor.snapshot_generation, loaded.snapshot_generation);
        assert_eq!(cursor.wal_segment_id, loaded.wal_segment_id);
        assert_eq!(cursor.wal_offset, loaded.wal_offset);

        // 清理
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_default_cursor() {
        let cursor = RecoveryCursor::default();
        assert_eq!(cursor.version, 1);
        assert_eq!(cursor.snapshot_generation, 0);
        assert_eq!(cursor.wal_segment_id, 0);
        assert_eq!(cursor.wal_offset, 0);
    }
}
