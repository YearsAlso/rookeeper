//! wal.rs
//!
//! 预写日志（WAL）模块，实现日志写入和滚动逻辑。

use std::fs::{File, OpenOptions};
use std::io::{Result as IoResult, Write};
use std::path::PathBuf;

/// 操作类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    /// 创建键值对
    Create = 1,
    /// 设置键值对
    Set = 2,
    /// 删除键值对
    Delete = 3,
    /// 设置 TTL
    SetTtl = 4,
    /// 获取分布式锁
    AcquireLock = 5,
    /// 释放分布式锁
    ReleaseLock = 6,
    /// 注册服务
    RegisterService = 7,
    /// 服务心跳
    ServiceHeartbeat = 8,
    /// 注销服务
    UnregisterService = 9,
}

impl OpType {
    /// 从 u8 值转换为 OpType
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(OpType::Create),
            2 => Some(OpType::Set),
            3 => Some(OpType::Delete),
            4 => Some(OpType::SetTtl),
            5 => Some(OpType::AcquireLock),
            6 => Some(OpType::ReleaseLock),
            7 => Some(OpType::RegisterService),
            8 => Some(OpType::ServiceHeartbeat),
            9 => Some(OpType::UnregisterService),
            _ => None,
        }
    }

    /// 转换为 u8 值
    pub fn to_u8(self) -> u8 {
        self as u8
    }
}

/// WAL 错误类型
#[derive(Debug)]
pub enum WalError {
    /// IO 错误
    IoError(std::io::Error),
    /// 校验和不匹配
    ChecksumMismatch { expected: u32, actual: u32 },
    /// 无效的操作类型
    InvalidOpType(u8),
    /// 编码错误
    EncodeError(String),
}

impl From<std::io::Error> for WalError {
    fn from(err: std::io::Error) -> Self {
        WalError::IoError(err)
    }
}

/// WAL 日志条目
///
/// 包含 Raft 日志 term、index 以及操作相关数据
#[derive(Debug, Clone)]
pub struct WalEntry {
    /// Raft term
    pub term: u64,
    /// 日志索引
    pub index: u64,
    /// 操作类型
    pub op_type: OpType,
    /// 路径
    pub path: String,
    /// 值（可选）
    pub value: Option<Vec<u8>>,
    /// TTL 秒数（可选）
    pub ttl_seconds: Option<u64>,
    /// CRC32 校验和
    pub crc32: u32,
}

impl WalEntry {
    /// 创建新的 WAL 条目
    pub fn new(
        term: u64,
        index: u64,
        op_type: OpType,
        path: String,
        value: Option<Vec<u8>>,
        ttl_seconds: Option<u64>,
    ) -> Self {
        // 计算校验和（不包含 crc32 字段本身）
        let crc32 = Self::compute_crc(term, index, op_type, &path, &value, ttl_seconds);
        Self {
            term,
            index,
            op_type,
            path,
            value,
            ttl_seconds,
            crc32,
        }
    }

    /// 计算校验和
    fn compute_crc(
        term: u64,
        index: u64,
        op_type: OpType,
        path: &str,
        value: &Option<Vec<u8>>,
        ttl_seconds: Option<u64>,
    ) -> u32 {
        use crc32fast::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&term.to_le_bytes());
        hasher.update(&index.to_le_bytes());
        hasher.update(&[op_type.to_u8()]);
        hasher.update(path.as_bytes());
        if let Some(ref v) = *value {
            hasher.update(v);
        }
        if let Some(ttl) = ttl_seconds {
            hasher.update(&ttl.to_le_bytes());
        }
        hasher.finalize()
    }

    /// 验证校验和
    pub fn verify_checksum(&self) -> bool {
        let expected = Self::compute_crc(
            self.term,
            self.index,
            self.op_type,
            &self.path,
            &self.value,
            self.ttl_seconds,
        );
        expected == self.crc32
    }

    /// 编码为字节数组
    ///
    /// 格式：term(8) + index(8) + op_type(1) + path_len(4) + path + value_len(4) + value + ttl(8) + crc32(4)
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // term
        buf.extend_from_slice(&self.term.to_le_bytes());
        // index
        buf.extend_from_slice(&self.index.to_le_bytes());
        // op_type
        buf.push(self.op_type.to_u8());
        // path
        let path_bytes = self.path.as_bytes();
        buf.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        buf.extend_from_slice(path_bytes);
        // value
        match &self.value {
            Some(v) => {
                buf.extend_from_slice(&(v.len() as u32).to_le_bytes());
                buf.extend_from_slice(v);
            }
            None => {
                buf.extend_from_slice(&0u32.to_le_bytes());
            }
        }
        // ttl
        match self.ttl_seconds {
            Some(ttl) => {
                buf.extend_from_slice(&1u32.to_le_bytes());
                buf.extend_from_slice(&ttl.to_le_bytes());
            }
            None => {
                buf.extend_from_slice(&0u32.to_le_bytes());
            }
        }
        // crc32
        buf.extend_from_slice(&self.crc32.to_le_bytes());

        buf
    }

    /// 从字节数组解码
    pub fn decode(buf: &[u8]) -> Result<Self, WalError> {
        let mut offset = 0;

        // term
        let term = u64::from_le_bytes(
            buf[offset..offset + 8]
                .try_into()
                .map_err(|e| WalError::EncodeError(format!("Failed to read term: {:?}", e)))?,
        );
        offset += 8;

        // index
        let index = u64::from_le_bytes(
            buf[offset..offset + 8]
                .try_into()
                .map_err(|e| WalError::EncodeError(format!("Failed to read index: {:?}", e)))?,
        );
        offset += 8;

        // op_type
        let op_type_val = buf[offset];
        let op_type = OpType::from_u8(op_type_val).ok_or(WalError::InvalidOpType(op_type_val))?;
        offset += 1;

        // path
        let path_len = u32::from_le_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|e| WalError::EncodeError(format!("Failed to read path_len: {:?}", e)))?,
        ) as usize;
        offset += 4;
        let path = String::from_utf8(buf[offset..offset + path_len].to_vec())
            .map_err(|e| WalError::EncodeError(format!("Failed to read path: {:?}", e)))?;
        offset += path_len;

        // value
        let value_len = u32::from_le_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|e| WalError::EncodeError(format!("Failed to read value_len: {:?}", e)))?,
        ) as usize;
        offset += 4;
        let value = if value_len > 0 {
            Some(buf[offset..offset + value_len].to_vec())
        } else {
            None
        };
        if value_len > 0 {
            offset += value_len;
        }

        // ttl
        let has_ttl = u32::from_le_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|e| WalError::EncodeError(format!("Failed to read has_ttl: {:?}", e)))?,
        );
        offset += 4;
        let ttl_seconds = if has_ttl == 1 {
            let ttl = u64::from_le_bytes(
                buf[offset..offset + 8]
                    .try_into()
                    .map_err(|e| WalError::EncodeError(format!("Failed to read ttl: {:?}", e)))?,
            );
            offset += 8;
            Some(ttl)
        } else {
            None
        };

        // crc32
        let crc32 = u32::from_le_bytes(
            buf[offset..offset + 4]
                .try_into()
                .map_err(|e| WalError::EncodeError(format!("Failed to read crc32: {:?}", e)))?,
        );

        let entry = Self {
            term,
            index,
            op_type,
            path,
            value,
            ttl_seconds,
            crc32,
        };

        // 验证校验和
        if !entry.verify_checksum() {
            return Err(WalError::ChecksumMismatch {
                expected: entry.crc32,
                actual: Self::compute_crc(
                    term,
                    index,
                    op_type,
                    &entry.path,
                    &entry.value,
                    entry.ttl_seconds,
                ),
            });
        }

        Ok(entry)
    }
}

/// WAL 写入器
///
/// 用于顺序写入日志条目到 WAL 段文件
pub struct WalWriter {
    /// 文件句柄
    file: File,
    /// 当前段 ID
    segment_id: u64,
    /// 当前偏移量
    offset: u64,
    /// 最大段大小（字节），超过后滚动
    max_segment_size: u64,
    /// 路径生成器
    path_cb: Box<dyn Fn(u64) -> PathBuf>,
}

impl WalWriter {
    /// 创建新的 WAL 写入器
    ///
    /// # Arguments
    /// * `path` - WAL 段文件路径
    /// * `segment_id` - 段 ID
    /// * `max_segment_size` - 最大段大小，超过后自动滚动
    pub fn new(path: impl Into<PathBuf>, segment_id: u64, max_segment_size: u64) -> IoResult<Self> {
        let path = path.into();
        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        let offset = file.metadata()?.len();
        let path_cb = Box::new(move |id: u64| {
            // 简单默认实现，实际使用 StorageLayout.wal_segment_path
            PathBuf::from(format!("wal-{:016}.log", id))
        });

        Ok(Self {
            file,
            segment_id,
            offset,
            max_segment_size,
            path_cb,
        })
    }

    /// 创建带有自定义路径生成器的 WAL 写入器
    pub fn with_path_cb(
        segment_id: u64,
        max_segment_size: u64,
        path_cb: impl Fn(u64) -> PathBuf + 'static,
    ) -> IoResult<Self> {
        let path = path_cb(segment_id);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;

        let offset = file.metadata()?.len();

        Ok(Self {
            file,
            segment_id,
            offset,
            max_segment_size,
            path_cb: Box::new(path_cb),
        })
    }

    /// 写入日志条目
    ///
    /// 返回写入后的偏移量
    pub fn write(&mut self, entry: &WalEntry) -> IoResult<u64> {
        let encoded = entry.encode();
        let len = encoded.len() as u64;

        self.file.write_all(&encoded)?;
        let written_offset = self.offset;
        self.offset += len;

        Ok(written_offset)
    }

    /// 同步日志到磁盘
    pub fn sync(&mut self) -> IoResult<()> {
        self.file.sync_all()
    }

    /// 检查是否需要滚动日志
    pub fn should_roll(&self) -> bool {
        self.offset >= self.max_segment_size
    }

    /// 获取当前段 ID
    pub fn segment_id(&self) -> u64 {
        self.segment_id
    }

    /// 获取当前偏移量
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// 滚动到新的 WAL 段
    ///
    /// 关闭当前段文件，创建新的段文件
    pub fn roll(&mut self) -> IoResult<()> {
        // 同步并关闭当前文件
        self.file.sync_all()?;

        // 创建新的段 ID
        self.segment_id += 1;

        // 打开新的段文件（直接重新赋值，旧的 File 会被自动关闭）
        let path = (self.path_cb)(self.segment_id);
        self.file = OpenOptions::new().create(true).append(true).open(&path)?;

        self.offset = 0;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optype_conversion() {
        assert_eq!(OpType::from_u8(1), Some(OpType::Create));
        assert_eq!(OpType::from_u8(2), Some(OpType::Set));
        assert_eq!(OpType::from_u8(3), Some(OpType::Delete));
        assert_eq!(OpType::from_u8(99), None);
    }

    #[test]
    fn test_wal_entry_encode_decode() {
        let entry = WalEntry::new(
            1,
            100,
            OpType::Set,
            "/test/path".to_string(),
            Some(vec![1, 2, 3]),
            Some(3600),
        );

        let encoded = entry.encode();
        let decoded = WalEntry::decode(&encoded).unwrap();

        assert_eq!(decoded.term, entry.term);
        assert_eq!(decoded.index, entry.index);
        assert_eq!(decoded.op_type, entry.op_type);
        assert_eq!(decoded.path, entry.path);
        assert_eq!(decoded.value, entry.value);
        assert_eq!(decoded.ttl_seconds, entry.ttl_seconds);
    }

    #[test]
    fn test_wal_entry_checksum() {
        let entry = WalEntry::new(1, 100, OpType::Create, "/test".to_string(), None, None);

        assert!(entry.verify_checksum());

        // 修改后校验失败
        let mut tampered = entry.clone();
        tampered.term = 999;
        assert!(!tampered.verify_checksum());
    }
}
