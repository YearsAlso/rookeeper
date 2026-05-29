//! 服务配置定义 - 服务器、存储、认证、可观测性等子系统的配置结构
//!
//! 设计原则：
//! - 所有配置项都有合理默认值，降低上手门槛
//! - 使用 PathBuf 而非 String 处理文件系统路径，避免跨平台兼容问题
//! - 分为多个子配置块，便于单独替换和测试

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// 传输模式选择，自动检测或手动指定
///
/// Auto 模式会根据操作系统选择最佳传输方式
/// 特定模式用于测试或特殊网络环境
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    /// 自动检测：根据操作系统选择最适合的 IPC 方式
    #[default]
    Auto,
    /// Unix 域套接字，Linux 默认，性能最佳
    UnixDomainSocket,
    /// 命名管道，Windows 推荐
    NamedPipe,
    /// 本地 TCP 回环，适用于跨容器/跨虚拟机场景
    LocalTcp,
}

/// 认证模式，用于控制访问权限
///
/// Phase 0 仅支持禁用认证，TokenFile 模式将在后续阶段实现
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    /// 认证禁用，允许所有操作（仅用于开发/内网环境）
    #[default]
    Disabled,
    /// 基于令牌文件的认证，令牌存储在配置目录
    TokenFile,
}

/// 日志输出格式，影响可读性和机器解析
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    /// JSON 格式，适合日志收集系统和结构化查询
    #[default]
    Json,
    /// 人类可读格式，适合终端调试
    Pretty,
}

/// 服务器运行时配置
///
/// 这些值决定了服务的行为和性能特征
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 节点名称，用于集群内标识和日志
    pub node_name: String,
    /// 主要传输方式，会优先尝试使用
    pub primary_transport: TransportMode,
    /// 回退传输方式，主要方式不可用时使用
    pub fallback_transport: TransportMode,
    /// 请求队列深度，超过此数量的请求会被拒绝
    /// 设置较大值可以应对突发流量
    pub request_queue_depth: usize,
    /// 会话超时时间（毫秒），客户端在此时间内未发送任何消息则会话失效
    pub session_timeout_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            // 默认节点名便于本地快速启动
            node_name: "local-node".to_string(),
            primary_transport: TransportMode::Auto,
            // TCP 作为回退确保跨平台可用性
            fallback_transport: TransportMode::LocalTcp,
            // 1024 是合理默认值，平衡内存和吞吐量
            request_queue_depth: 1024,
            // 30 秒超时足够容忍短暂网络抖动
            session_timeout_ms: 30_000,
        }
    }
}

/// 存储层配置，定义了数据持久化的行为和限制
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    /// 数据根目录，所有子目录都基于此路径计算
    pub root_dir: PathBuf,
    /// WAL（预写日志）目录，相对于 root_dir
    pub wal_dir: String,
    /// 快照目录，用于存储定期生成的状态快照
    pub snapshot_dir: String,
    /// 集群状态目录，存储元数据而非业务数据
    pub state_dir: String,
    /// 分布式锁文件路径，用于确保单实例运行
    pub lock_file: String,
    /// 单个 WAL 段的最大字节数，超过后创建新段
    /// 限制段大小便于管理和恢复
    pub max_wal_segment_bytes: u64,
    /// 快照间隔命令数，经过此数量操作后触发快照
    /// 平衡内存使用和恢复时间
    pub snapshot_interval_commands: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            // 默认使用项目目录下 data/ 目录
            root_dir: PathBuf::from("./data"),
            wal_dir: "wal".to_string(),
            snapshot_dir: "snapshot".to_string(),
            state_dir: "state".to_string(),
            lock_file: "rookeeper.lock".to_string(),
            // 64MB 是平衡值：足够大减少文件数量，又足够小便于管理
            max_wal_segment_bytes: 64 * 1024 * 1024,
            // 1 万次操作后创建快照，平衡内存和启动时间
            snapshot_interval_commands: 10_000,
        }
    }
}

/// 认证配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    /// 认证模式
    pub mode: AuthMode,
    /// 令牌文件路径（用于 TokenFile 模式）
    pub token_file: PathBuf,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Disabled,
            // 默认路径便于容器化部署
            token_file: PathBuf::from("./config/tokens.toml"),
        }
    }
}

/// 可观测性配置，日志和监控相关设置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// 日志输出格式
    pub log_format: LogFormat,
    /// 日志级别，控制详细程度
    pub log_level: String,
    /// 指标服务监听地址，用于 Prometheus 等监控系统拉取
    pub metrics_bind: String,
    /// 是否启用指标收集，开启会增加少量开销
    pub enable_metrics: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            // JSON 格式便于日志收集系统处理
            log_format: LogFormat::Json,
            log_level: "info".to_string(),
            // 默认仅监听本地，安全性考量
            metrics_bind: "127.0.0.1:9642".to_string(),
            enable_metrics: true,
        }
    }
}

/// 兼容性配置，控制行为兼容性开关
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityConfig {
    /// 是否启用路径规范化（统一分隔符、去除 ..）
    pub normalize_paths: bool,
    /// 是否允许反斜杠作为路径分隔符输入
    pub allow_backslash_paths: bool,
}

impl Default for CompatibilityConfig {
    fn default() -> Self {
        Self {
            // 默认开启，方便从其他系统迁移
            normalize_paths: true,
            allow_backslash_paths: true,
        }
    }
}

/// 完整的服务配置，聚合所有子系统配置
///
/// 这是配置加载的顶层结构，通常从配置文件或环境变量读取
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServiceConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    pub observability: ObservabilityConfig,
    pub compatibility: CompatibilityConfig,
}
