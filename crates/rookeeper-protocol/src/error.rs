//! 错误码定义 - 协调服务可能遇到的各种错误类型
//!
//! 错误码采用 u16 确保可以跨语言/跨平台传递
//! 255 保留给内部未预期错误，避免泄露敏感信息

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 协调服务错误码，采用枚举确保类型安全
///
/// 错误码按功能分组（0-99 成功/业务错误，100-199 权限/安全，200+ 系统错误）
/// 这样便于日志分析和监控告警规则配置。
///
/// 枚举值直接对应 u16 编号（通过 serde 序列化控制）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// 操作成功完成
    Success = 0,
    /// 指定的路径不存在
    PathNotFound = 1,
    /// 路径已存在，无法创建重复节点
    PathAlreadyExists = 2,
    /// 版本冲突，乐观锁检查失败（并发修改）
    VersionConflict = 3,
    /// 权限不足，ACL 检查拒绝访问
    PermissionDenied = 4,
    /// 路径格式无效，不符合规范
    InvalidPath = 5,
    /// 会话已过期或不存在，需要重新建立连接
    SessionExpired = 6,
    /// 锁已被其他客户端持有，请求方需等待重试
    LockBusy = 7,
    /// 资源耗尽（如磁盘空间、内存限制）
    ResourceExhausted = 8,
    /// 存储数据损坏，需要人工介入修复
    StorageCorruption = 9,
    /// 传输层不可用（网络问题、目标服务未启动）
    TransportUnavailable = 10,
    /// 协议版本不支持，客户端/服务端版本不匹配
    UnsupportedVersion = 11,
    // ─────── Phase 2 新增错误码 ───────
    /// 锁获取超时
    LockTimeout = 12,
    /// 会话不存在
    SessionNotFound = 13,
    /// 服务已注册
    ServiceAlreadyRegistered = 14,
    /// 服务不存在
    ServiceNotFound = 15,
    /// 锁未被当前会话持有
    LockNotHeld = 16,
    /// 锁不存在
    LockNotFound = 17,
    /// Watch 不存在或已过期
    WatchNotFound = 18,
    /// 内部未预期错误，仅用于日志记录不返回给客户端
    Internal = 255,
}

impl ErrorCode {
    /// 转换为原始 u16 值，便于序列化和传输
    pub const fn as_u16(self) -> u16 {
        self as u16
    }

    /// 从原始 u16 值构造错误码（用于协议解析）
    ///
    /// 如果值超出已知范围，返回 `ErrorCode::Internal`。
    pub const fn from_u16(v: u16) -> Self {
        match v {
            0 => ErrorCode::Success,
            1 => ErrorCode::PathNotFound,
            2 => ErrorCode::PathAlreadyExists,
            3 => ErrorCode::VersionConflict,
            4 => ErrorCode::PermissionDenied,
            5 => ErrorCode::InvalidPath,
            6 => ErrorCode::SessionExpired,
            7 => ErrorCode::LockBusy,
            8 => ErrorCode::ResourceExhausted,
            9 => ErrorCode::StorageCorruption,
            10 => ErrorCode::TransportUnavailable,
            11 => ErrorCode::UnsupportedVersion,
            12 => ErrorCode::LockTimeout,
            13 => ErrorCode::SessionNotFound,
            14 => ErrorCode::ServiceAlreadyRegistered,
            15 => ErrorCode::ServiceNotFound,
            16 => ErrorCode::LockNotHeld,
            17 => ErrorCode::LockNotFound,
            18 => ErrorCode::WatchNotFound,
            _ => ErrorCode::Internal,
        }
    }
}

/// 结构化错误类型，包含错误码和可读消息
///
/// 消息字段用于调试和问题诊断，但不应用于程序逻辑判断
/// 因为消息文本可能因版本而变化
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code:?}: {message}")]
pub struct RookeeperError {
    /// 错误分类码
    pub code: ErrorCode,
    /// 详细描述信息，供运维人员排查
    pub message: String,
}

impl RookeeperError {
    /// 构造新的错误实例
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
