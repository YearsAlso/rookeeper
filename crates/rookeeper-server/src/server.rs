//! server.rs
//!
//! TCP Binary Server 实现，处理客户端请求和管理连接。
//!
//! 设计原则：
//! - 使用 tokio 异步处理并发连接
//! - 帧格式：24 字节头部 + 变长载荷（bincode 序列化）
//! - 请求路由到 TreeKv 处理
//! - 支持 Watch 持久订阅推送

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex, MutexGuard};

use rookeeper_protocol::config::RuntimeMode;
use rookeeper_protocol::error::ErrorCode;
use rookeeper_protocol::model::{NodePath, SessionId};
use rookeeper_protocol::wire::{
    FrameHeader, HealthCheckResponse, MetricsResponse, RequestFrame, RequestKind,
    ResponseFrame, FRAME_HEADER_LEN,
};

use crate::tree_kv::{TreeKv, TreeKvEvent};
use crate::watch::{WatchEvent, WatchManager};

/// 服务器错误类型
#[derive(Debug)]
pub enum ServerError {
    /// IO 错误
    Io(std::io::Error),
    /// 协议解析错误
    Protocol(String),
    /// 内部错误
    Internal(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::Io(e) => write!(f, "IO error: {}", e),
            ServerError::Protocol(s) => write!(f, "Protocol error: {}", s),
            ServerError::Internal(s) => write!(f, "Internal error: {}", s),
        }
    }
}

impl Error for ServerError {}

impl From<std::io::Error> for ServerError {
    fn from(err: std::io::Error) -> Self {
        ServerError::Io(err)
    }
}

/// 请求载荷定义
mod requests {
    use bytes::Bytes;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct GetRequest {
        pub path: NodePath,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct CreateRequest {
        pub path: NodePath,
        pub value: Vec<u8>,
        pub ttl_seconds: Option<u64>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct SetRequest {
        pub path: NodePath,
        pub value: Vec<u8>,
        pub expected_version: Option<u64>,
        pub ttl_seconds: Option<u64>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DeleteRequest {
        pub path: NodePath,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ListRequest {
        pub path: NodePath,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct WatchRequest {
        pub path: NodePath,
        pub recursive: bool,
    }
}

/// TCP 服务器
pub struct RookeeperServer {
    /// TreeKv 存储
    tree_kv: Arc<Mutex<TreeKv>>,
    /// 运行时模式
    mode: RuntimeMode,
    /// Watch 管理器
    watch_manager: Arc<Mutex<WatchManager>>,
    /// TCP 监听器
    listener: Option<TcpListener>,
    /// 服务器启动时间
    start_time: Instant,
    /// 版本信息
    version: String,
    /// 活跃连接
    connections: Arc<Mutex<HashMap<SessionId, broadcast::Sender<WatchEvent>>>>,
}

impl RookeeperServer {
    /// 创建新的服务器实例
    pub async fn new(config: rookeeper_protocol::config::ServiceConfig) -> Result<Self, ServerError> {
        let mode = config.runtime_mode();
        let bind_addr = config.tcp_bind.clone();

        // 创建 TCP 监听器
        let listener = TcpListener::bind(&bind_addr).await?;
        tracing::info!("TCP server listening on {}", bind_addr);

        let server = Self {
            tree_kv: Arc::new(Mutex::new(TreeKv::new())),
            mode,
            watch_manager: Arc::new(Mutex::new(WatchManager::new())),
            listener: Some(listener),
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            connections: Arc::new(Mutex::new(HashMap::new())),
        };

        Ok(server)
    }

    /// 运行服务器，接受连接并处理请求
    pub async fn run(&self) -> Result<(), ServerError> {
        let listener = self.listener.as_ref().ok_or_else(|| {
            ServerError::Internal("Server not initialized".to_string())
        })?;

        tracing::info!("Rookeeper server started in {:?} mode", self.mode);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::debug!("New connection from {}", addr);
                    let tree_kv = Arc::clone(&self.tree_kv);
                    let watch_manager = Arc::clone(&self.watch_manager);
                    let start_time = self.start_time;
                    let version = self.version.clone();
                    let connections = Arc::clone(&self.connections);

                    // 清理 TTL（异步不阻塞）
                    {
                        let tree_kv_clone = Arc::clone(&tree_kv);
                        tokio::spawn(async move {
                            let mut tree = tree_kv_clone.lock().await;
                            let expired = tree.expired_nodes();
                            for path in expired {
                                let _ = tree.delete(&path, "system");
                            }
                        });
                    }

                    // 处理连接
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream,
                            tree_kv,
                            watch_manager,
                            start_time,
                            version,
                            connections,
                        )
                        .await
                        {
                            tracing::error!("Connection handler error: {:?}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {:?}", e);
                }
            }
        }
    }

    /// 处理单个客户端连接
    async fn handle_connection(
        mut stream: TcpStream,
        tree_kv: Arc<Mutex<TreeKv>>,
        watch_manager: Arc<Mutex<WatchManager>>,
        start_time: Instant,
        version: String,
        connections: Arc<Mutex<HashMap<SessionId, broadcast::Sender<WatchEvent>>>>,
    ) -> Result<(), ServerError> {
        let session_id = rand_u64();
        tracing::debug!("Session {} established", session_id);

        // 将连接添加到活跃连接列表
        let (event_tx, _) = broadcast::channel(256);
        {
            let mut conns: MutexGuard<'_, HashMap<SessionId, broadcast::Sender<WatchEvent>>> = connections.lock().await;
            conns.insert(session_id, event_tx.clone());
        }

        // 循环处理请求直到连接关闭
        loop {
            // 读取帧头部
            let mut header_buf = [0u8; FRAME_HEADER_LEN];
            let n = stream.read(&mut header_buf).await?;
            if n == 0 {
                tracing::debug!("Session {} disconnected", session_id);
                break;
            }
            if n != FRAME_HEADER_LEN {
                return Err(ServerError::Protocol("Incomplete header".to_string()));
            }

            // 解析帧头部
            let header = Self::parse_header(&header_buf)?;

            // 读取载荷
            let mut payload_buf = vec![0u8; header.payload_len as usize];
            if header.payload_len > 0 {
                stream.read_exact(&mut payload_buf).await?;
            }
            let payload = Bytes::from(payload_buf);

            // 构建请求帧
            let request = RequestFrame {
                header,
                payload,
            };

            // 处理请求并生成响应
            let response = Self::process_request(
                &request,
                session_id,
                &tree_kv,
                &watch_manager,
                start_time,
                &version,
            )
            .await?;

            // 发送响应
            let response_bytes = Self::serialize_response(&response)?;
            stream.write_all(&response_bytes).await?;
            stream.flush().await?;
        }

        // 清理连接
        {
            let mut conns: MutexGuard<'_, HashMap<SessionId, broadcast::Sender<WatchEvent>>> = connections.lock().await;
            conns.remove(&session_id);
        }
        watch_manager.lock().await.unregister_session(session_id);

        Ok(())
    }

    /// 解析帧头部
    fn parse_header(buf: &[u8; FRAME_HEADER_LEN]) -> Result<FrameHeader, ServerError> {
        let mut offset = 0;

        let version = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        offset += 2;

        let request_kind = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        offset += 2;

        let flags = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        offset += 2;

        let reserved = u16::from_le_bytes([buf[offset], buf[offset + 1]]);
        offset += 2;

        let request_id = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);
        offset += 4;

        let session_id = u64::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
            buf[offset + 4],
            buf[offset + 5],
            buf[offset + 6],
            buf[offset + 7],
        ]);
        offset += 8;

        let payload_len = u32::from_le_bytes([
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]);

        Ok(FrameHeader {
            version,
            request_kind,
            flags,
            reserved,
            request_id,
            session_id,
            payload_len,
        })
    }

    /// 处理请求并生成响应
    async fn process_request(
        request: &RequestFrame,
        session_id: SessionId,
        tree_kv: &Arc<Mutex<TreeKv>>,
        watch_manager: &Arc<Mutex<WatchManager>>,
        start_time: Instant,
        version: &str,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;

        // 处理不同类型的请求
        match request.header.request_kind {
            // Get 请求
            k if k == RequestKind::Get as u16 => {
                Self::handle_get(request, tree_kv).await
            }
            // Create 请求
            k if k == RequestKind::Create as u16 => {
                Self::handle_create(request, tree_kv, watch_manager).await
            }
            // Set 请求
            k if k == RequestKind::Set as u16 => {
                Self::handle_set(request, tree_kv, watch_manager).await
            }
            // Delete 请求
            k if k == RequestKind::Delete as u16 => {
                Self::handle_delete(request, tree_kv, watch_manager).await
            }
            // List 请求
            k if k == RequestKind::List as u16 => {
                Self::handle_list(request, tree_kv).await
            }
            // Watch 请求
            k if k == RequestKind::Watch as u16 => {
                Self::handle_watch(request, session_id, watch_manager).await
            }
            // HealthCheck 请求
            k if k == RequestKind::HealthCheck as u16 => {
                Self::handle_health_check(tree_kv, start_time, version).await
            }
            // MetricsRequest 请求
            k if k == RequestKind::MetricsRequest as u16 => {
                Self::handle_metrics(tree_kv).await
            }
            // 未知请求类型
            _ => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::new(),
            }),
        }
    }

    /// 处理 Get 请求
    async fn handle_get(
        request: &RequestFrame,
        tree_kv: &Arc<Mutex<TreeKv>>,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;
        let session_subject = "anonymous";

        match bincode::deserialize(request.payload.as_ref()) {
            Ok(req) => {
                let get_req: requests::GetRequest = req;
                let tree = tree_kv.lock().await;

                match tree.get(&get_req.path, session_subject) {
                    Ok(node) => {
                        let response_payload =
                            bincode::serialize(&node.value.to_vec()).map_err(|e| {
                                ServerError::Internal(format!("Serialization error: {}", e))
                            })?;
                        Ok(ResponseFrame {
                            request_id,
                            status: ErrorCode::Success,
                            payload: Bytes::from(response_payload),
                        })
                    }
                    Err(e) => Ok(ResponseFrame {
                        request_id,
                        status: ErrorCode::from(e),
                        payload: Bytes::new(),
                    }),
                }
            }
            Err(e) => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::from(format!("Parse error: {}", e)),
            }),
        }
    }

    /// 处理 Create 请求
    async fn handle_create(
        request: &RequestFrame,
        tree_kv: &Arc<Mutex<TreeKv>>,
        watch_manager: &Arc<Mutex<WatchManager>>,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;
        let session_subject = "anonymous";

        match bincode::deserialize(request.payload.as_ref()) {
            Ok(req) => {
                let create_req: requests::CreateRequest = req;
                let mut tree = tree_kv.lock().await;

                match tree.create(
                    &create_req.path,
                    Bytes::from(create_req.value),
                    session_subject,
                    create_req.ttl_seconds,
                ) {
                    Ok(()) => {
                        // 触发 Watch 事件
                        let event = TreeKvEvent::NodeCreated(create_req.path.clone());
                        let sessions = watch_manager
                            .lock()
                            .await
                            .matching_subscriptions(&create_req.path);
                        for sid in sessions {
                            if let Some(we) = WatchEvent::from_tree_kv_event(&event, sid) {
                                let _ = watch_manager.lock().await.broadcast_event(we);
                            }
                        }

                        Ok(ResponseFrame {
                            request_id,
                            status: ErrorCode::Success,
                            payload: Bytes::new(),
                        })
                    }
                    Err(e) => Ok(ResponseFrame {
                        request_id,
                        status: ErrorCode::from(e),
                        payload: Bytes::new(),
                    }),
                }
            }
            Err(e) => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::from(format!("Parse error: {}", e)),
            }),
        }
    }

    /// 处理 Set 请求
    async fn handle_set(
        request: &RequestFrame,
        tree_kv: &Arc<Mutex<TreeKv>>,
        watch_manager: &Arc<Mutex<WatchManager>>,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;
        let session_subject = "anonymous";

        match bincode::deserialize(request.payload.as_ref()) {
            Ok(req) => {
                let set_req: requests::SetRequest = req;
                let mut tree = tree_kv.lock().await;

                match tree.set(
                    &set_req.path,
                    Bytes::from(set_req.value),
                    set_req.expected_version,
                    session_subject,
                    set_req.ttl_seconds,
                ) {
                    Ok(()) => {
                        // 触发 Watch 事件
                        let event = TreeKvEvent::NodeUpdated(set_req.path.clone());
                        let sessions = watch_manager
                            .lock()
                            .await
                            .matching_subscriptions(&set_req.path);
                        for sid in sessions {
                            if let Some(we) = WatchEvent::from_tree_kv_event(&event, sid) {
                                let _ = watch_manager.lock().await.broadcast_event(we);
                            }
                        }

                        Ok(ResponseFrame {
                            request_id,
                            status: ErrorCode::Success,
                            payload: Bytes::new(),
                        })
                    }
                    Err(e) => Ok(ResponseFrame {
                        request_id,
                        status: ErrorCode::from(e),
                        payload: Bytes::new(),
                    }),
                }
            }
            Err(e) => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::from(format!("Parse error: {}", e)),
            }),
        }
    }

    /// 处理 Delete 请求
    async fn handle_delete(
        request: &RequestFrame,
        tree_kv: &Arc<Mutex<TreeKv>>,
        watch_manager: &Arc<Mutex<WatchManager>>,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;
        let session_subject = "anonymous";

        match bincode::deserialize(request.payload.as_ref()) {
            Ok(req) => {
                let delete_req: requests::DeleteRequest = req;
                let mut tree = tree_kv.lock().await;

                match tree.delete(&delete_req.path, session_subject) {
                    Ok(()) => {
                        // 触发 Watch 事件
                        let event = TreeKvEvent::NodeDeleted(delete_req.path.clone());
                        let sessions = watch_manager
                            .lock()
                            .await
                            .matching_subscriptions(&delete_req.path);
                        for sid in sessions {
                            if let Some(we) = WatchEvent::from_tree_kv_event(&event, sid) {
                                let _ = watch_manager.lock().await.broadcast_event(we);
                            }
                        }

                        Ok(ResponseFrame {
                            request_id,
                            status: ErrorCode::Success,
                            payload: Bytes::new(),
                        })
                    }
                    Err(e) => Ok(ResponseFrame {
                        request_id,
                        status: ErrorCode::from(e),
                        payload: Bytes::new(),
                    }),
                }
            }
            Err(e) => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::from(format!("Parse error: {}", e)),
            }),
        }
    }

    /// 处理 List 请求
    async fn handle_list(
        request: &RequestFrame,
        tree_kv: &Arc<Mutex<TreeKv>>,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;
        let session_subject = "anonymous";

        match bincode::deserialize(request.payload.as_ref()) {
            Ok(req) => {
                let list_req: requests::ListRequest = req;
                let tree = tree_kv.lock().await;

                match tree.list(&list_req.path, session_subject) {
                    Ok(children) => {
                        let response_payload = bincode::serialize(&children).map_err(|e| {
                            ServerError::Internal(format!("Serialization error: {}", e))
                        })?;
                        Ok(ResponseFrame {
                            request_id,
                            status: ErrorCode::Success,
                            payload: Bytes::from(response_payload),
                        })
                    }
                    Err(e) => Ok(ResponseFrame {
                        request_id,
                        status: ErrorCode::from(e),
                        payload: Bytes::new(),
                    }),
                }
            }
            Err(e) => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::from(format!("Parse error: {}", e)),
            }),
        }
    }

    /// 处理 Watch 请求
    async fn handle_watch(
        request: &RequestFrame,
        session_id: SessionId,
        watch_manager: &Arc<Mutex<WatchManager>>,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = request.header.request_id;

        match bincode::deserialize(request.payload.as_ref()) {
            Ok(req) => {
                let watch_req: requests::WatchRequest = req;
                let mut manager = watch_manager.lock().await;

                let subscription_id = manager.register_watch(session_id, watch_req.path, watch_req.recursive);

                let response_payload = bincode::serialize(&subscription_id).map_err(|e| {
                    ServerError::Internal(format!("Serialization error: {}", e))
                })?;

                Ok(ResponseFrame {
                    request_id,
                    status: ErrorCode::Success,
                    payload: Bytes::from(response_payload),
                })
            }
            Err(e) => Ok(ResponseFrame {
                request_id,
                status: ErrorCode::Internal,
                payload: Bytes::from(format!("Parse error: {}", e)),
            }),
        }
    }

    /// 处理 HealthCheck 请求
    async fn handle_health_check(
        tree_kv: &Arc<Mutex<TreeKv>>,
        start_time: Instant,
        version: &str,
    ) -> Result<ResponseFrame, ServerError> {
        let request_id = 0; // HealthCheck 请求没有 request_id

        let tree = tree_kv.lock().await;
        let node_count = tree.len();
        let uptime = start_time.elapsed().as_secs();

        let mode_str = match RuntimeMode::default() {
            RuntimeMode::Memory => "memory",
            RuntimeMode::Persistent => "persistent",
        };

        let response = HealthCheckResponse {
            healthy: true,
            mode: mode_str.to_string(),
            version: version.to_string(),
            uptime_seconds: uptime,
            node_count,
        };

        let response_payload = bincode::serialize(&response).map_err(|e| {
            ServerError::Internal(format!("Serialization error: {}", e))
        })?;

        Ok(ResponseFrame {
            request_id,
            status: ErrorCode::Success,
            payload: Bytes::from(response_payload),
        })
    }

    /// 处理 MetricsRequest 请求
    async fn handle_metrics(tree_kv: &Arc<Mutex<TreeKv>>) -> Result<ResponseFrame, ServerError> {
        let request_id = 0;

        let tree = tree_kv.lock().await;
        let node_count = tree.len();
        let command_id = tree.command_id();

        // 生成 Prometheus text format 指标
        let prometheus_text = format!(
            "# HELP rookeeper_node_count Number of nodes in the tree\n\
             # TYPE rookeeper_node_count gauge\n\
             rookeeper_node_count {}\n\
             # HELP rookeeper_command_id Current command ID\n\
             # TYPE rookeeper_command_id counter\n\
             rookeeper_command_id {}\n",
            node_count, command_id
        );

        let response = MetricsResponse {
            prometheus_text,
        };

        let response_payload = bincode::serialize(&response).map_err(|e| {
            ServerError::Internal(format!("Serialization error: {}", e))
        })?;

        Ok(ResponseFrame {
            request_id,
            status: ErrorCode::Success,
            payload: Bytes::from(response_payload),
        })
    }

    /// 序列化响应帧
    fn serialize_response(response: &ResponseFrame) -> Result<Vec<u8>, ServerError> {
        let payload_bytes = bincode::serialize(&response.payload).map_err(|e| {
            ServerError::Internal(format!("Payload serialization error: {}", e))
        })?;

        let header = FrameHeader::new(
            RequestKind::Get, // 响应帧的 kind 字段不用于路由
            response.request_id,
            0, // session_id
            payload_bytes.len() as u32,
        );

        let mut result = Vec::with_capacity(FRAME_HEADER_LEN + payload_bytes.len());
        result.extend_from_slice(&header.version.to_le_bytes());
        result.extend_from_slice(&header.request_kind.to_le_bytes());
        result.extend_from_slice(&header.flags.to_le_bytes());
        result.extend_from_slice(&header.reserved.to_le_bytes());
        result.extend_from_slice(&header.request_id.to_le_bytes());
        result.extend_from_slice(&header.session_id.to_le_bytes());
        result.extend_from_slice(&header.payload_len.to_le_bytes());
        result.extend_from_slice(&payload_bytes);

        Ok(result)
    }
}

/// 生成随机会话 ID
fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (now.as_nanos() as u64) ^ (std::process::id() as u64 * 1000000007)
}