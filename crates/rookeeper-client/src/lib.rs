use rookeeper_platform::default_endpoint;
use rookeeper_protocol::config::{ServiceConfig, TransportMode};
use rookeeper_protocol::wire::{FrameHeader, RequestKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientOptions {
    pub endpoint: String,
    pub auth_token: Option<String>,
    pub timeout_ms: u64,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            endpoint: default_endpoint("rookeeper"),
            auth_token: None,
            timeout_ms: 3_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientBootstrap {
    pub options: ClientOptions,
}

impl ClientBootstrap {
    pub fn from_config(config: &ServiceConfig) -> Self {
        let endpoint = match config.server.primary_transport {
            TransportMode::Auto => default_endpoint("rookeeper"),
            TransportMode::UnixDomainSocket => "unix:///tmp/rookeeper.sock".to_string(),
            TransportMode::NamedPipe => r"pipe://./pipe/rookeeper".to_string(),
            TransportMode::LocalTcp => "tcp://127.0.0.1:9641".to_string(),
        };

        Self {
            options: ClientOptions {
                endpoint,
                auth_token: None,
                timeout_ms: config.server.session_timeout_ms,
            },
        }
    }

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
