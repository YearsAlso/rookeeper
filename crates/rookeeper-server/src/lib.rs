use std::path::Path;

use anyhow::{bail, Result};
use rookeeper_platform::default_endpoint;
use rookeeper_protocol::config::{ServiceConfig, TransportMode};
use rookeeper_storage::StorageLayout;

#[derive(Debug, Clone)]
pub struct ServerBootstrap {
    pub config: ServiceConfig,
    pub storage_layout: StorageLayout,
    pub endpoint: String,
}

impl ServerBootstrap {
    pub fn from_config(config: ServiceConfig) -> Self {
        let endpoint = match config.server.primary_transport {
            TransportMode::Auto => default_endpoint("rookeeper"),
            TransportMode::UnixDomainSocket => "unix:///tmp/rookeeper.sock".to_string(),
            TransportMode::NamedPipe => r"pipe://./pipe/rookeeper".to_string(),
            TransportMode::LocalTcp => "tcp://127.0.0.1:9641".to_string(),
        };

        let storage_layout = StorageLayout::from_root(&config.storage.root_dir);

        Self {
            config,
            storage_layout,
            endpoint,
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "node={} endpoint={} data_root={}",
            self.config.server.node_name,
            self.endpoint,
            self.storage_layout.root.display()
        )
    }
}

pub fn load_config(path: Option<&Path>) -> Result<ServiceConfig> {
    match path {
        Some(path) if path.exists() => {
            bail!(
                "config file parsing is intentionally deferred after Phase 0; use the default model and template at config/rookeeper.default.toml"
            )
        }
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
