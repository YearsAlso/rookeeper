//! session.rs
//!
//! 会话管理模块：管理客户端连接生命周期、租约和临时节点清理。
//!
//! 设计原则：
//! - 租约（Lease）机制：会话必须在超时时间内续约，否则视为断开
//! - 临时节点（Ephemeral）生命周期与会话绑定，会话结束时自动清理
//! - 心跳（Heartbeat）用于租约续期，避免频繁重建连接
//! - 单机实现：会话存储在内存中，服务重启后需要客户端重连

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::time::interval;

use rookeeper_protocol::error::ErrorCode;
use rookeeper_protocol::model::SessionId;

/// 会话状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// 活跃会话，正常工作
    Active,
    /// 会话已过期，等待清理
    Expired,
    /// 已关闭
    Closed,
}

/// 会话元数据
#[derive(Debug, Clone)]
pub struct SessionMeta {
    /// 会话唯一标识
    pub id: SessionId,
    /// 创建时间
    pub created_at: Instant,
    /// 租约截止时间
    pub lease_deadline: Instant,
    /// 租约超时（毫秒）
    pub timeout_ms: u64,
    /// 会话状态
    pub state: SessionState,
    /// 会话关联的临时节点路径
    pub ephemeral_nodes: Vec<String>,
}

impl SessionMeta {
    /// 检查租约是否已过期
    pub fn is_expired(&self) -> bool {
        Instant::now() > self.lease_deadline
    }

    /// 续期租约
    pub fn renew(&mut self) {
        self.lease_deadline = Instant::now() + Duration::from_millis(self.timeout_ms);
    }
}

/// 会话管理器
///
/// 管理所有活跃会话的生命周期：
/// - 创建会话、续期心跳、清理过期会话
/// - 跟踪会话持有的临时节点，断开时触发清理
#[derive(Debug)]
pub struct SessionManager {
    /// 会话表：SessionId → SessionMeta
    sessions: RwLock<HashMap<SessionId, SessionMeta>>,
    /// 默认租约超时（毫秒）
    default_timeout_ms: u64,
    /// 会话计数器
    next_session_id: RwLock<SessionId>,
    /// 已关闭会话 ID（用于去重）
    closed_sessions: RwLock<Vec<SessionId>>,
}

impl SessionManager {
    /// 创建会话管理器
    pub fn new(default_timeout_ms: u64) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            default_timeout_ms,
            next_session_id: RwLock::new(1),
            closed_sessions: RwLock::new(Vec::new()),
        }
    }

    /// 创建新会话
    ///
    /// 返回分配的 SessionId
    pub async fn create_session(&self) -> SessionId {
        let session_id = {
            let mut counter = self.next_session_id.write().await;
            let id = *counter;
            *counter += 1;
            id
        };

        let session = SessionMeta {
            id: session_id,
            created_at: Instant::now(),
            lease_deadline: Instant::now() + Duration::from_millis(self.default_timeout_ms),
            timeout_ms: self.default_timeout_ms,
            state: SessionState::Active,
            ephemeral_nodes: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);
        session_id
    }

    /// 心跳续期
    ///
    /// 如果会话存在且未过期，重置租约截止时间。
    /// 如果会话不存在或已过期，返回错误。
    pub async fn heartbeat(&self, session_id: SessionId) -> Result<(), ErrorCode> {
        let mut sessions = self.sessions.write().await;
        let session = sessions
            .get_mut(&session_id)
            .ok_or(ErrorCode::SessionNotFound)?;

        if session.state != SessionState::Active {
            return Err(ErrorCode::SessionExpired);
        }

        if session.is_expired() {
            session.state = SessionState::Expired;
            return Err(ErrorCode::SessionExpired);
        }

        session.renew();
        Ok(())
    }

    /// 注册临时节点到会话
    ///
    /// 当会话创建一个 ephemeral 节点时调用，
    /// 方便会话断开时统一清理。
    pub async fn register_ephemeral_node(&self, session_id: SessionId, node_path: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.ephemeral_nodes.push(node_path.to_string());
        }
    }

    /// 获取会话持有的所有临时节点路径
    pub async fn get_session_ephemerals(&self, session_id: SessionId) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id)
            .map(|s| s.ephemeral_nodes.clone())
            .unwrap_or_default()
    }

    /// 关闭会话
    ///
    /// 将会话状态设为 Closed，返回需要清理的临时节点路径。
    /// 不执行实际的节点删除（由调用方通过 TreeKv 删除）。
    pub async fn close_session(&self, session_id: SessionId) -> Result<Vec<String>, ErrorCode> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .get_mut(&session_id)
            .ok_or(ErrorCode::SessionNotFound)?;

        if session.state == SessionState::Closed {
            return Err(ErrorCode::SessionNotFound);
        }

        session.state = SessionState::Closed;
        let nodes = session.ephemeral_nodes.clone();

        // 记录已关闭的会话
        let mut closed = self.closed_sessions.write().await;
        closed.push(session_id);

        Ok(nodes)
    }

    /// 检查会话是否存活
    pub async fn is_alive(&self, session_id: SessionId) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id)
            .map(|s| s.state == SessionState::Active && !s.is_expired())
            .unwrap_or(false)
    }

    /// 扫描并清理所有过期的会话
    ///
    /// 返回需要清理的临时节点映射：SessionId → [node_paths]
    pub async fn collect_expired_sessions(&self) -> HashMap<SessionId, Vec<String>> {
        let mut sessions = self.sessions.write().await;
        let mut expired = HashMap::new();

        for (id, session) in sessions.iter_mut() {
            if session.state == SessionState::Active && session.is_expired() {
                session.state = SessionState::Expired;
                expired.insert(*id, session.ephemeral_nodes.clone());
            }
        }

        expired
    }

    /// 获取活跃会话数
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.state == SessionState::Active && !s.is_expired())
            .count()
    }

    /// 获取所有会话摘要
    pub async fn session_summary(&self) -> Vec<(SessionId, SessionState)> {
        let sessions = self.sessions.read().await;
        sessions
            .iter()
            .map(|(id, s)| (*id, s.state.clone()))
            .collect()
    }

    /// 清理已关闭的会话记录
    ///
    /// 减少内存占用。
    pub async fn purge_closed_sessions(&self) {
        let mut sessions = self.sessions.write().await;
        let closed: Vec<SessionId> = {
            let c = self.closed_sessions.read().await;
            c.clone()
        };
        for id in closed {
            sessions.remove(&id);
        }
        let mut c = self.closed_sessions.write().await;
        c.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let manager = SessionManager::new(5000);
        let id = manager.create_session().await;
        assert_eq!(id, 1);
        assert!(manager.is_alive(id).await);

        let id2 = manager.create_session().await;
        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_heartbeat_renews_lease() {
        let manager = SessionManager::new(100); // 100ms timeout
        let id = manager.create_session().await;

        // 短暂等待后心跳应该成功
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(manager.heartbeat(id).await.is_ok());
    }

    #[tokio::test]
    async fn test_session_expires_without_heartbeat() {
        let manager = SessionManager::new(50); // 50ms timeout
        let id = manager.create_session().await;

        // 等待租约过期
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(!manager.is_alive(id).await);
    }

    #[tokio::test]
    async fn test_close_session_returns_ephemerals() {
        let manager = SessionManager::new(5000);
        let id = manager.create_session().await;

        manager
            .register_ephemeral_node(id, "/ephemeral/node1")
            .await;
        manager
            .register_ephemeral_node(id, "/ephemeral/node2")
            .await;

        let nodes = manager.close_session(id).await.unwrap();
        assert_eq!(nodes.len(), 2);
        assert!(!manager.is_alive(id).await);
    }

    #[tokio::test]
    async fn test_collect_expired_sessions() {
        let manager = SessionManager::new(30);
        let id = manager.create_session().await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        let expired = manager.collect_expired_sessions().await;
        assert!(expired.contains_key(&id));
        assert_eq!(expired[&id].len(), 0); // no ephemerals registered
    }

    #[tokio::test]
    async fn test_double_close_session() {
        let manager = SessionManager::new(5000);
        let id = manager.create_session().await;

        manager.close_session(id).await.unwrap();
        let result = manager.close_session(id).await;
        assert!(result.is_err());
    }
}
