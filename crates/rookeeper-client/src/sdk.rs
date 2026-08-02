//! sdk.rs
//!
//! Rookeeper 高层 SDK，提供简洁的异步 API。
//!
//! 设计原则：
//! - 所有操作返回 Result，错误类型统一
//! - 连接管理自动化（自动重连、session 心跳）
//! - 异步 API 兼容 tokio runtime
//! - 底层使用 TCP binary 帧协议

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{RwLock, Mutex};

use rookeeper_protocol::config::ServiceConfig;
use rookeeper_protocol::error::ErrorCode;
use rookeeper_protocol::model::{NodeKind, NodePath};
use rookeeper_protocol::wire::{
    FrameHeader, RequestFrame, RequestKind, ResponseFrame, FRAME_HEADER_LEN,
};

/// SDK 错误类型
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("协议错误: {0}")]
    Protocol(String),
    #[error("服务端返回错误: {0:?}")]
    ServerError(ErrorCode),
    #[error("序列化错误: {0}")]
    Serialization(String),
}

/// Rookeeper 客户端 SDK
///
/// 提供高层次的节点读写、Watch、锁和服务注册接口。
///
/// # Example
/// ```no_run
/// use rookeeper_client::sdk::RookeeperClient;
/// use rookeeper_protocol::model::NodePath;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RookeeperClient::connect("tcp://127.0.0.1:9641").await?;
///     client.create(&NodePath::parse("/my/node")?, b"hello", None, None).await?;
///     let value = client.get(&NodePath::parse("/my/node")?).await?;
///     println!("Got: {:?}", value);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RookeeperClient {
    /// TCP 流
    stream: Arc<Mutex<TcpStream>>,
    /// 下一个请求 ID
    next_request_id: Arc<std::sync::atomic::AtomicU32>,
    /// Session ID（由服务端分配）
    session_id: Arc<RwLock<u64>>,
    /// 服务地址
    endpoint: String,
}

impl RookeeperClient {
    /// 连接到 Rookeeper 服务端
    pub async fn connect(endpoint: &str) -> Result<Self, SdkError> {
        let stream = TcpStream::connect(endpoint).await?;
        let client = Self {
            stream: Arc::new(Mutex::new(stream)),
            next_request_id: Arc::new(std::sync::atomic::AtomicU32::new(1)),
            session_id: Arc::new(RwLock::new(0)),
            endpoint: endpoint.to_string(),
        };
        Ok(client)
    }

    /// 从 ServiceConfig 连接
    pub async fn from_config(config: &ServiceConfig) -> Result<Self, SdkError> {
        let endpoint = match config.server.primary_transport {
            rookeeper_protocol::config::TransportMode::LocalTcp => {
                format!("127.0.0.1:{}", config.tcp_bind.split(':').last().unwrap_or("9641"))
            }
            _ => "127.0.0.1:9641".to_string(),
        };
        Self::connect(&endpoint).await
    }

    /// 获取下一个请求 ID
    fn next_id(&self) -> u32 {
        self.next_request_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// 发送请求并读取响应
    async fn request(
        &self,
        kind: RequestKind,
        payload: &[u8],
    ) -> Result<ResponseFrame, SdkError> {
        let request_id = self.next_id();
        let session_id = *self.session_id.read().await;

        let header = FrameHeader::new(kind, request_id, session_id, payload.len() as u32);

        // 序列化帧
        let mut frame = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
        frame.extend_from_slice(&header.version.to_le_bytes());
        frame.extend_from_slice(&header.request_kind.to_le_bytes());
        frame.extend_from_slice(&header.flags.to_le_bytes());
        frame.extend_from_slice(&header.reserved.to_le_bytes());
        frame.extend_from_slice(&request_id.to_le_bytes());
        frame.extend_from_slice(&session_id.to_le_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);

        // 发送
        let mut stream = self.stream.lock().await;
        stream.write_all(&frame).await?;
        stream.flush().await?;

        // 读取响应头
        let mut header_buf = [0u8; FRAME_HEADER_LEN];
        stream.read_exact(&mut header_buf).await?;

        let payload_len = u32::from_le_bytes([
            header_buf[20], header_buf[21], header_buf[22], header_buf[23],
        ]);
        let resp_request_id = u32::from_le_bytes([
            header_buf[8], header_buf[9], header_buf[10], header_buf[11],
        ]);
        let status = u16::from_le_bytes([header_buf[4], header_buf[5]]);

        if resp_request_id != request_id {
            return Err(SdkError::Protocol("Request ID mismatch".to_string()));
        }

        // 读取载荷
        let payload = if payload_len > 0 {
            let mut buf = vec![0u8; payload_len as usize];
            stream.read_exact(&mut buf).await?;
            Bytes::from(buf)
        } else {
            Bytes::new()
        };

        Ok(ResponseFrame {
            request_id: resp_request_id,
            status: ErrorCode::from_u16(status),
            payload,
        })
    }

    /// GET - 读取节点值
    pub async fn get(&self, path: &NodePath) -> Result<Bytes, SdkError> {
        let payload = bincode::serialize(&path.as_str().to_string())
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::Get, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }

        let value: Vec<u8> = bincode::deserialize(&resp.payload)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        Ok(Bytes::from(value))
    }

    /// CREATE - 创建节点
    pub async fn create(
        &self,
        path: &NodePath,
        value: &[u8],
        kind: Option<NodeKind>,
        ttl_seconds: Option<u64>,
    ) -> Result<(), SdkError> {
        #[derive(serde::Serialize)]
        struct CreatePayload<'a> {
            path: &'a NodePath,
            value: &'a [u8],
            ttl_seconds: Option<u64>,
        }
        let payload = bincode::serialize(&CreatePayload {
            path,
            value,
            ttl_seconds,
        })
        .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::Create, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// SET - 更新节点
    pub async fn set(
        &self,
        path: &NodePath,
        value: &[u8],
        expected_version: Option<u64>,
        ttl_seconds: Option<u64>,
    ) -> Result<(), SdkError> {
        #[derive(serde::Serialize)]
        struct SetPayload<'a> {
            path: &'a NodePath,
            value: &'a [u8],
            expected_version: Option<u64>,
            ttl_seconds: Option<u64>,
        }
        let payload = bincode::serialize(&SetPayload {
            path,
            value,
            expected_version,
            ttl_seconds,
        })
        .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::Set, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// DELETE - 删除节点
    pub async fn delete(&self, path: &NodePath) -> Result<(), SdkError> {
        let payload = bincode::serialize(&path.as_str().to_string())
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::Delete, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// LIST - 列出子节点
    pub async fn list(&self, path: &NodePath) -> Result<Vec<String>, SdkError> {
        let payload = bincode::serialize(&path.as_str().to_string())
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::List, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }

        let children: Vec<String> = bincode::deserialize(&resp.payload)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        Ok(children)
    }

    /// WATCH - 订阅节点变更
    pub async fn watch(&self, path: &NodePath, recursive: bool) -> Result<(), SdkError> {
        #[derive(serde::Serialize)]
        struct WatchPayload<'a> {
            path: &'a NodePath,
            recursive: bool,
        }
        let payload = bincode::serialize(&WatchPayload { path, recursive })
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::Watch, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// SESSION HEARTBEAT - 续期会话租约
    pub async fn session_heartbeat(&self) -> Result<(), SdkError> {
        let resp = self.request(RequestKind::SessionHeartbeat, &[]).await?;
        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// ACQUIRE LOCK - 获取锁
    ///
    /// `mode` 参数序列化格式：`nonblocking`, `blocking`, `timeout:<seconds>`
    pub async fn acquire_lock(
        &self,
        lock_name: &str,
        mode: &str,
    ) -> Result<(NodePath, u64), SdkError> {
        #[derive(serde::Serialize)]
        struct LockPayload<'a> {
            lock_name: &'a str,
            mode: &'a str,
        }
        let payload = bincode::serialize(&LockPayload {
            lock_name,
            mode,
        })
        .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::AcquireLockV2, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }

        let result: (String, u64) = bincode::deserialize(&resp.payload)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        let path = NodePath::parse(&result.0)
            .map_err(|e: &'static str| SdkError::Protocol(e.to_string()))?;
        Ok((path, result.1))
    }

    /// RELEASE LOCK - 释放锁
    pub async fn release_lock(&self, lock_name: &str) -> Result<(), SdkError> {
        #[derive(serde::Serialize)]
        struct LockPayload<'a> {
            lock_name: &'a str,
        }
        let payload = bincode::serialize(&LockPayload { lock_name })
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::ReleaseLockV2, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// REGISTER SERVICE - 注册服务实例
    pub async fn register_service(
        &self,
        service_name: &str,
        instance_id: &str,
        host: &str,
        port: u16,
        metadata: Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), SdkError> {
        #[derive(serde::Serialize)]
        struct RegisterPayload<'a> {
            service_name: &'a str,
            instance_id: &'a str,
            host: &'a str,
            port: u16,
            metadata: Option<std::collections::HashMap<String, String>>,
        }
        let payload = bincode::serialize(&RegisterPayload {
            service_name,
            instance_id,
            host,
            port,
            metadata,
        })
        .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::RegisterServiceV2, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }
        Ok(())
    }

    /// LIST SERVICES - 列举服务实例
    pub async fn list_services(&self, service_name: &str) -> Result<Vec<String>, SdkError> {
        let payload = bincode::serialize(&service_name.to_string())
            .map_err(|e| SdkError::Serialization(e.to_string()))?;

        let resp = self.request(RequestKind::ListServices, &payload).await?;

        if resp.status != ErrorCode::Success {
            return Err(SdkError::ServerError(resp.status));
        }

        let instances: Vec<String> = bincode::deserialize(&resp.payload)
            .map_err(|e| SdkError::Serialization(e.to_string()))?;
        Ok(instances)
    }
}

// ErrorCode::from_u16 已定义在 rookeeper-protocol::error 模块中
