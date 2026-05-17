use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    #[default]
    Auto,
    UnixDomainSocket,
    NamedPipe,
    LocalTcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    Disabled,
    TokenFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LogFormat {
    #[default]
    Json,
    Pretty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub node_name: String,
    pub primary_transport: TransportMode,
    pub fallback_transport: TransportMode,
    pub request_queue_depth: usize,
    pub session_timeout_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            node_name: "local-node".to_string(),
            primary_transport: TransportMode::Auto,
            fallback_transport: TransportMode::LocalTcp,
            request_queue_depth: 1024,
            session_timeout_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageConfig {
    pub root_dir: PathBuf,
    pub wal_dir: String,
    pub snapshot_dir: String,
    pub state_dir: String,
    pub lock_file: String,
    pub max_wal_segment_bytes: u64,
    pub snapshot_interval_commands: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("./data"),
            wal_dir: "wal".to_string(),
            snapshot_dir: "snapshot".to_string(),
            state_dir: "state".to_string(),
            lock_file: "rookeeper.lock".to_string(),
            max_wal_segment_bytes: 64 * 1024 * 1024,
            snapshot_interval_commands: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthConfig {
    pub mode: AuthMode,
    pub token_file: PathBuf,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::Disabled,
            token_file: PathBuf::from("./config/tokens.toml"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_format: LogFormat,
    pub log_level: String,
    pub metrics_bind: String,
    pub enable_metrics: bool,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Json,
            log_level: "info".to_string(),
            metrics_bind: "127.0.0.1:9642".to_string(),
            enable_metrics: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityConfig {
    pub normalize_paths: bool,
    pub allow_backslash_paths: bool,
}

impl Default for CompatibilityConfig {
    fn default() -> Self {
        Self {
            normalize_paths: true,
            allow_backslash_paths: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServiceConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub auth: AuthConfig,
    pub observability: ObservabilityConfig,
    pub compatibility: CompatibilityConfig,
}
