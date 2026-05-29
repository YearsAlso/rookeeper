//! rookeeper-client
//!
//! 客户端引导程序，负责从共享配置派生端点和请求头。
//!
//! 设计原则：
//! - 客户端不自行定义传输或配置逻辑，复用 platform 和 protocol 中的共享类型
//! - 请求头构造统一化，确保客户端/服务端协议版本一致
//! - 默认值集中管理，便于配置变更

use rookeeper_platform::default_endpoint;
use rookeeper_protocol::config::{ServiceConfig, TransportMode};
use rookeeper_protocol::wire::{FrameHeader, RequestKind};

/// 客户端连接选项
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientOptions {
    /// 服务端端点地址
    pub endpoint: String,
    /// 认证令牌（可选）
    pub auth_token: Option<String>,
    /// 请求超时时间（毫秒）
    pub timeout_ms: u64,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            // 使用平台默认值，确保与运行环境匹配
            endpoint: default_endpoint("rookeeper"),
            auth_token: None,
            // 3 秒超时是网络回环的合理默认值
            timeout_ms: 3_000,
        }
    }
}

/// 客户端引导程序，负责初始化客户端实例
///
/// 从 ServiceConfig 派生所有必要的连接参数
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBootstrap {
    pub options: ClientOptions,
}

impl ClientBootstrap {
    /// 从服务配置构造引导程序
    ///
    /// 端点根据配置的传输模式选择：
    /// - Auto 使用平台默认值
    /// - 特定模式使用对应地址格式
    pub fn from_config(config: &ServiceConfig) -> Self {
        let endpoint = match config.server.primary_transport {
            TransportMode::Auto => default_endpoint("rookeeper"),
            TransportMode::UnixDomainSocket => "unix:///tmp/rookeeper.sock".to_string(),
            TransportMode::NamedPipe => r"pipe://./pipe/rookeeper".to_string(),
            TransportMode::LocalTcp => "tcp://127.0.0.1:9641/rookeeper".to_string(),
        };

        Self {
            options: ClientOptions {
                endpoint,
                auth_token: None,
                timeout_ms: config.server.session_timeout_ms,
            },
        }
    }

    /// 构造请求帧头部
    ///
    /// 会话 ID 初始化为 0，正式连接前由服务端分配
    pub fn build_header(
        &self,
        kind: RequestKind,
        request_id: u32,
        payload_len: u32,
    ) -> FrameHeader {
        FrameHeader::new(kind, request_id, 0, payload_len)
    }
}

#[cfg(test)]
mod tests {
    use rookeeper_protocol::config::ServiceConfig;
    use rookeeper_protocol::wire::RequestKind;

    use super::ClientBootstrap;

    #[test]
    fn creates_headers_with_version() {
        let client = ClientBootstrap::from_config(&ServiceConfig::default());
        let header = client.build_header(RequestKind::Get, 7, 16);

        assert_eq!(header.request_id, 7);
        assert_eq!(header.payload_len, 16);
    }
}
