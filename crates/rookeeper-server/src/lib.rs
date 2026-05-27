//! rookeeper-server
//! 
//! 服务端引导程序，负责从配置初始化运行时和存储布局。
//! 
//! 设计原则：
//! - 服务端不自行定义传输或存储逻辑，复用 platform 和 storage 中的共享类型
//! - 配置加载有意推迟到 Phase 1，确保 Phase 0 保持最小可用状态
//! - 引导流程清晰分离，便于测试和监控

use std::path::Path;

use anyhow::{bail, Result};
use rookeeper_platform::default_endpoint;
use rookeeper_protocol::config::{ServiceConfig, TransportMode};
use rookeeper_storage::StorageLayout;

/// 服务端引导程序，包含运行时初始化所需的所有组件
#[derive(Debug, Clone)]
pub struct ServerBootstrap {
    /// 服务配置
    pub config: ServiceConfig,
    /// 存储布局
    pub storage_layout: StorageLayout,
    /// 监听端点
    pub endpoint: String,
}

impl ServerBootstrap {
    /// 从服务配置构造引导程序
    /// 
    /// 端点选择逻辑与客户端保持一致，确保通信双方匹配
    pub fn from_config(config: ServiceConfig) -> Self {
        let endpoint = match config.server.primary_transport {
            TransportMode::Auto => default_endpoint("rookeeper"),
            TransportMode::UnixDomainSocket => "unix:///tmp/rookeeper.sock".to_string(),
            TransportMode::NamedPipe => r"pipe://./pipe/rookeeper".to_string(),
            TransportMode::LocalTcp => "tcp://127.0.0.1:9641".to_string(),
        };

        // 从配置派生存储布局，保持一致性
        let storage_layout = StorageLayout::from_root(&config.storage.root_dir);

        Self {
            config,
            storage_layout,
            endpoint,
        }
    }

    /// 生成运行时摘要信息，用于日志和监控
    pub fn summary(&self) -> String {
        format!(
            "node={} endpoint={} data_root={}",
            self.config.server.node_name,
            self.endpoint,
            self.storage_layout.root.display()
        )
    }
}

/// 加载服务配置
/// 
/// Phase 0 阶段：故意拒绝外部配置文件，确保基线稳定
/// Phase 1 将实现完整的配置解析逻辑
pub fn load_config(path: Option<&Path>) -> Result<ServiceConfig> {
    match path {
        // 如果指定了配置文件，明确告知用户当前不支持
        // 这是有意设计，避免用户误以为已实现但实际使用了默认配置
        Some(path) if path.exists() => {
            bail!(
                "config file parsing is intentionally deferred after Phase 0; use the default model and template at config/rookeeper.default.toml"
            )
        }
        // 无配置文件时使用默认配置，便于快速启动
        _ => Ok(ServiceConfig::default()),
    }
}

#[cfg(test)]
mod tests {
    use rookeeper_protocol::config::ServiceConfig;

    use super::ServerBootstrap;

    #[test]
    fn derives_storage_layout_from_config() {
        let bootstrap = ServerBootstrap::from_config(ServiceConfig::default());
        assert!(bootstrap.storage_layout.root.ends_with("data"));
    }
}