#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    Linux,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcTransport {
    UnixDomainSocket,
    NamedPipe,
    LocalTcp,
}

pub fn current_os() -> OperatingSystem {
    if cfg!(target_os = "windows") {
        OperatingSystem::Windows
    } else if cfg!(target_os = "linux") {
        OperatingSystem::Linux
    } else {
        OperatingSystem::Other
    }
}

pub fn default_ipc_transport() -> IpcTransport {
    match current_os() {
        OperatingSystem::Windows => IpcTransport::NamedPipe,
        OperatingSystem::Linux => IpcTransport::UnixDomainSocket,
        OperatingSystem::Other => IpcTransport::LocalTcp,
    }
}

pub fn default_endpoint(service_name: &str) -> String {
    match default_ipc_transport() {
        IpcTransport::UnixDomainSocket => format!("unix:///tmp/{service_name}.sock"),
        IpcTransport::NamedPipe => format!(r"pipe://./pipe/{service_name}"),
        IpcTransport::LocalTcp => "tcp://127.0.0.1:9641".to_string(),
    }
}

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
