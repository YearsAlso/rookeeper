//! rookeeper-platform
//!
//! 平台抽象层，封装操作系统相关行为。
//!
//! 设计原则：
//! - 平台差异代码集中于此，避免散落到业务逻辑中
//! - 通过枚举而非条件编译暴露平台能力，便于类型检查
//! - 默认选择最适合当前 OS 的 IPC 传输方式

/// 操作系统枚举，用于平台特定行为决策
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    /// Windows 系统
    Windows,
    /// Linux 系统
    Linux,
    /// 其他类 Unix 系统或未知
    Other,
}

/// IPC 传输方式，影响性能和连接方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcTransport {
    /// Unix 域套接字 - Linux 最优，性能高且无需网络栈
    UnixDomainSocket,
    /// 命名管道 - Windows 推荐，跨会话可靠
    NamedPipe,
    /// 本地 TCP 回环 - 跨平台兼容，适用于容器环境
    LocalTcp,
}

/// 检测当前运行的操作系统
///
/// 使用编译时条件确定，零运行时开销
pub fn current_os() -> OperatingSystem {
    if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "linux") {
        OperatingSystem::Linux
    } else {
        OperatingSystem::Other
    }
}

/// 获取当前操作系统推荐的默认 IPC 传输方式
///
/// 选择依据：
/// - Linux 使用 Unix 域套接字，性能最优
/// - Windows 使用命名管道，行为最可靠
/// - 其他系统回退到 TCP 回环，确保可用性
pub fn default_ipc_transport() -> IpcTransport {
    match current_os() {
        OperatingSystem::Windows => IpcTransport::NamedPipe,
        OperatingSystem::Linux => IpcTransport::UnixDomainSocket,
        OperatingSystem::Other => IpcTransport::LocalTcp,
    }
}

/// 构造服务的默认端点地址
///
/// 格式根据传输方式不同：
/// - Unix 域套接字：`unix:///tmp/{service_name}.sock`
/// - 命名管道：`pipe://./pipe/{service_name}`
/// - TCP：`tcp://127.0.0.1:9641`
pub fn default_endpoint(service_name: &str) -> String {
    match default_ipc_transport() {
        IpcTransport::UnixDomainSocket => format!("unix:///tmp/{service_name}.sock"),
        IpcTransport::NamedPipe => format!(r"pipe://./pipe/{service_name}"),
        IpcTransport::LocalTcp => format!("tcp://127.0.0.1:9641/{service_name}"),
    }
}

/// 获取系统服务名称
///
/// 用于端点构造和日志标识
pub fn system_service_name() -> &'static str {
    "rookeeper"
}

#[cfg(test)]
mod tests {
    use super::{default_endpoint, system_service_name};

    #[test]
    fn exposes_service_name() {
        assert_eq!(system_service_name(), "rookeeper");
        assert!(default_endpoint("rookeeper").contains("rookeeper"));
    }
}
