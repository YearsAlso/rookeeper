//! Wire 协议定义 - 二进制帧格式和请求/事件类型
//!
//! 设计原则：
//! - 固定 24 字节头部确保解析效率，无需遍历可变长头部
//! - 请求和事件类型分离，避免混淆
//! - 版本字段支持协议演进和向后兼容

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;
use crate::model::SessionId;

/// 当前协议版本，用于握手时协商
pub const PROTOCOL_VERSION_V1: u16 = 1;

/// 帧头部固定长度，确保二进制解析的确定性
///
/// 结构布局（24字节）：
/// - version: u16 (2字节) - 协议版本
/// - request_kind: u16 (2字节) - 请求类型
/// - flags: u16 (2字节) - 标志位
/// - reserved: u16 (2字节) - 保留字段
/// - request_id: u32 (4字节) - 请求标识，用于响应匹配
/// - session_id: u64 (8字节) - 会话标识
/// - payload_len: u32 (4字节) - 载荷长度
pub const FRAME_HEADER_LEN: usize = 24;

/// 请求类型枚举，定义客户端可以发起的操作
///
/// 枚举值直接对应 u16 编号（通过 serde 序列化控制）。
/// Phase 1: 1-12, Phase 2: 13-20
///
/// 这些类型直接对应协调服务的核心能力：
/// - 读写节点数据
/// - 监听变化事件
/// - 分布式锁管理
/// - 服务注册与发现
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestKind {
    /// 读取节点数据
    Get = 1,
    /// 创建新节点
    Create = 2,
    /// 更新节点内容
    Set = 3,
    /// 删除节点
    Delete = 4,
    /// 列出子节点
    List = 5,
    /// 订阅节点变化事件
    Watch = 6,
    /// 获取分布式锁（Phase 1 保留编号，Phase 2 实现）
    AcquireLock = 7,
    /// 释放分布式锁（Phase 1 保留编号，Phase 2 实现）
    ReleaseLock = 8,
    /// 注册服务实例（Phase 1 保留编号，Phase 2 实现）
    RegisterService = 9,
    /// 查询服务状态（Phase 1 保留编号，Phase 2 实现）
    GetStatus = 10,
    /// 健康检查
    HealthCheck = 11,
    /// 指标请求
    MetricsRequest = 12,
    // ─────── Phase 2 新增请求类型 ───────
    /// 取消订阅
    CancelWatch = 13,
    /// 获取锁（阻塞/非阻塞/超时）
    AcquireLockV2 = 14,
    /// 释放锁
    ReleaseLockV2 = 15,
    /// 注册服务实例
    RegisterServiceV2 = 16,
    /// 注销服务实例
    UnregisterService = 17,
    /// 列举服务实例
    ListServices = 18,
    /// 会话心跳保活
    SessionHeartbeat = 19,
    /// 查询锁状态
    GetLockState = 20,
}

/// 事件类型枚举，定义服务端可以推送的通知
///
/// 枚举值直接对应 u16 编号（通过 serde 序列化控制）。
/// Phase 1: 1-8, Phase 2: 9-15
///
/// 事件是单向的，由服务端主动推送给已订阅的客户端
/// 用于实现 watch 机制和状态同步
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    /// 节点被创建
    NodeCreated = 1,
    /// 节点内容被更新
    NodeUpdated = 2,
    /// 节点被删除
    NodeDeleted = 3,
    /// 子节点列表发生变化
    ChildrenChanged = 4,
    /// 锁被获取（Phase 1 保留编号）
    LockAcquired = 5,
    /// 锁被释放（Phase 1 保留编号）
    LockReleased = 6,
    /// 服务注册成功（Phase 1 保留编号）
    ServiceRegistered = 7,
    /// 服务注销（Phase 1 保留编号）
    ServiceUnregistered = 8,
    // ─────── Phase 2 新增事件类型 ───────
    /// Watch 订阅超时
    WatchExpired = 9,
    /// 锁获取成功
    LockAcquiredV2 = 10,
    /// 锁释放
    LockReleasedV2 = 11,
    /// 锁被强制回收（会话断开/租约到期）
    LockRevoked = 12,
    /// 服务注册成功
    ServiceRegisteredV2 = 13,
    /// 服务注销
    ServiceUnregisteredV2 = 14,
    /// 服务实例下线
    ServiceOffline = 15,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// 服务是否健康
    pub healthy: bool,
    /// 运行时模式 ("memory" or "persistent")
    pub mode: String,
    /// 版本信息
    pub version: String,
    /// 服务运行时间（秒）
    pub uptime_seconds: u64,
    /// 节点数量
    pub node_count: usize,
}

/// 指标响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsResponse {
    /// Prometheus text format 指标数据
    pub prometheus_text: String,
}

/// 帧头部结构，描述每个请求/响应帧的元信息
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameHeader {
    /// 协议版本号，用于版本协商
    pub version: u16,
    /// 请求类型（RequestKind 的 u16 值）
    pub request_kind: u16,
    /// 标志位，用于扩展（如压缩、加密）
    pub flags: u16,
    /// 保留字段，暂时未使用
    pub reserved: u16,
    /// 请求唯一标识，用于匹配请求和响应
    pub request_id: u32,
    /// 会话标识，建立连接时分配
    pub session_id: SessionId,
    /// 载荷数据长度（字节）
    pub payload_len: u32,
}

impl FrameHeader {
    /// 构造新的帧头部
    ///
    /// 版本固定为 PROTOCOL_VERSION_V1
    /// flags 和 reserved 初始化为 0，后续可扩展
    pub fn new(
        kind: RequestKind,
        request_id: u32,
        session_id: SessionId,
        payload_len: u32,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION_V1,
            request_kind: kind as u16,
            flags: 0,
            reserved: 0,
            request_id,
            session_id,
            payload_len,
        }
    }
}

/// 请求帧，包含头部和载荷数据
///
/// 载荷格式由 request_kind 决定，通常是序列化后的请求结构
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestFrame {
    /// 帧头部元信息
    pub header: FrameHeader,
    /// 序列化后的请求数据
    pub payload: Bytes,
}

/// 响应帧，包含请求标识、状态和可选的载荷数据
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseFrame {
    /// 对应的请求标识
    pub request_id: u32,
    /// 操作结果状态
    pub status: ErrorCode,
    /// 序列化后的响应数据（可选）
    pub payload: Bytes,
}
