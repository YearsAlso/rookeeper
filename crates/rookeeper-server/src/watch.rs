//! watch.rs
//!
//! Watch 持久订阅系统实现，支持客户端订阅节点变更事件。
//!
//! 设计原则：
//! - 订阅通过 TCP 连接主动推送事件，而非客户端轮询
//! - 订阅持久有效（不超时），直到连接断开
//! - 支持 glob 模式匹配路径

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::broadcast;

use rookeeper_protocol::model::{NodePath, SessionId};

/// 路径匹配模式
#[derive(Debug, Clone)]
pub enum PathPattern {
    /// 精确路径匹配
    Exact(NodePath),
    /// Glob 模式匹配（如 /parent/* 或 /a/b/**）
    Glob(String),
}

impl PathPattern {
    /// 检查路径是否匹配此模式
    pub fn matches(&self, path: &NodePath) -> bool {
        match self {
            PathPattern::Exact(p) => p == path,
            PathPattern::Glob(pattern) => {
                // 简单的 glob 匹配实现
                // * 匹配单层路径段
                // ** 匹配多层路径
                let path_str = path.as_str();
                Self::glob_match(pattern, path_str)
            }
        }
    }

    /// 简单的 glob 匹配
    fn glob_match(pattern: &str, path: &str) -> bool {
        if pattern == "**" {
            return true;
        }

        if pattern.ends_with("/**") {
            let prefix = &pattern[..pattern.len() - 3];
            return path.starts_with(prefix) || path == &prefix[..prefix.len() - 1];
        }

        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            if path.starts_with(prefix) {
                let remainder = &path[prefix.len()..];
                // 匹配单层：remainder 为空（精确前缀匹配）或 remainder 是单层路径段（去掉前导 / 后不包含 /）
                let clean_remainder = remainder.strip_prefix('/').unwrap_or(remainder);
                return clean_remainder.is_empty() || !clean_remainder.contains('/');
            }
            return false;
        }

        // 精确匹配（带前缀检测）
        if pattern.contains('*') {
            return false;
        }

        path == pattern
    }
}

/// Watch 事件
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// 事件类型
    pub kind: WatchEventKind,
    /// 触发事件的路径
    pub path: NodePath,
    /// 订阅的会话 ID
    pub session_id: SessionId,
}

/// Watch 事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    /// 节点被创建
    NodeCreated,
    /// 节点内容被更新
    NodeUpdated,
    /// 节点被删除
    NodeDeleted,
    /// 子节点列表发生变化
    ChildrenChanged,
}

/// 单一订阅
#[derive(Debug, Clone)]
pub struct Subscription {
    /// 订阅的路径模式
    pub pattern: PathPattern,
    /// 是否递归监听子节点
    pub recursive: bool,
}

/// Watch 管理器
///
/// 管理所有客户端订阅，并负责将事件推送给匹配的订阅者
#[derive(Debug)]
pub struct WatchManager {
    /// 所有订阅：session_id -> [订阅列表]
    subscriptions: HashMap<SessionId, Vec<Subscription>>,
    /// 广播通道，用于分发事件
    event_tx: broadcast::Sender<WatchEvent>,
}

impl Default for WatchManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WatchManager {
    /// 创建新的 Watch 管理器
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            subscriptions: HashMap::new(),
            event_tx,
        }
    }

    /// 注册新的订阅
    ///
    /// 返回订阅 ID
    pub fn register_watch(
        &mut self,
        session_id: SessionId,
        path: NodePath,
        recursive: bool,
    ) -> usize {
        let subscriptions = self.subscriptions.entry(session_id).or_default();

        // 如果是递归模式，转换为 glob 模式
        let pattern = if recursive {
            PathPattern::Glob(format!("{}/**", path.as_str().trim_end_matches('/')))
        } else {
            PathPattern::Exact(path)
        };

        let id = subscriptions.len();
        subscriptions.push(Subscription { pattern, recursive });
        id
    }

    /// 取消订阅
    pub fn unregister_watch(&mut self, session_id: SessionId, subscription_id: usize) -> bool {
        if let Some(subs) = self.subscriptions.get_mut(&session_id) {
            if subscription_id < subs.len() {
                subs.remove(subscription_id);
                return true;
            }
        }
        false
    }

    /// 取消会话的所有订阅
    pub fn unregister_session(&mut self, session_id: SessionId) {
        self.subscriptions.remove(&session_id);
    }

    /// 检查路径变更是否匹配任何订阅
    ///
    /// 返回匹配订阅的会话 ID 列表
    pub fn matching_subscriptions(&self, path: &NodePath) -> Vec<SessionId> {
        let mut sessions = Vec::new();

        for (session_id, subscriptions) in &self.subscriptions {
            for sub in subscriptions {
                if sub.pattern.matches(path) {
                    sessions.push(*session_id);
                    break;
                }
            }
        }

        sessions
    }

    /// 广播事件到所有匹配订阅的客户端
    pub fn broadcast_event(&self, event: WatchEvent) {
        let _ = self.event_tx.send(event);
    }

    /// 获取广播接收器
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.event_tx.subscribe()
    }

    /// 获取活跃订阅数量
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.values().map(|s| s.len()).sum()
    }

    /// 获取会话数量
    pub fn session_count(&self) -> usize {
        self.subscriptions.len()
    }
}

/// 事件转换工具
impl WatchEvent {
    /// 从 TreeKvEvent 转换
    pub fn from_tree_kv_event(
        event: &crate::tree_kv::TreeKvEvent,
        session_id: SessionId,
    ) -> Option<Self> {
        match event {
            crate::tree_kv::TreeKvEvent::NodeCreated(path) => Some(Self {
                kind: WatchEventKind::NodeCreated,
                path: path.clone(),
                session_id,
            }),
            crate::tree_kv::TreeKvEvent::NodeUpdated(path) => Some(Self {
                kind: WatchEventKind::NodeUpdated,
                path: path.clone(),
                session_id,
            }),
            crate::tree_kv::TreeKvEvent::NodeDeleted(path) => Some(Self {
                kind: WatchEventKind::NodeDeleted,
                path: path.clone(),
                session_id,
            }),
            crate::tree_kv::TreeKvEvent::NodeExpired(_) => None, // TTL 过期不触发 Watch
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_pattern_match() {
        let pattern = PathPattern::Exact(NodePath::parse("/test/node").unwrap());
        let matching = NodePath::parse("/test/node").unwrap();
        let non_matching = NodePath::parse("/test/other").unwrap();

        assert!(pattern.matches(&matching));
        assert!(!pattern.matches(&non_matching));
    }

    #[test]
    fn test_glob_pattern_single_star() {
        let pattern = PathPattern::Glob("/parent/*".to_string());
        let matching = NodePath::parse("/parent/child1").unwrap();
        let non_matching = NodePath::parse("/parent/child1/grandchild").unwrap();

        assert!(pattern.matches(&matching));
        assert!(!pattern.matches(&non_matching));
    }

    #[test]
    fn test_glob_pattern_double_star() {
        let pattern = PathPattern::Glob("/parent/**".to_string());
        let matching1 = NodePath::parse("/parent/child1").unwrap();
        let matching2 = NodePath::parse("/parent/child1/grandchild").unwrap();
        let non_matching = NodePath::parse("/other/node").unwrap();

        assert!(pattern.matches(&matching1));
        assert!(pattern.matches(&matching2));
        assert!(!pattern.matches(&non_matching));
    }

    #[test]
    fn test_watch_manager_registration() {
        let mut manager = WatchManager::new();

        let session_id = 123;
        manager.register_watch(
            session_id,
            NodePath::parse("/test").unwrap(),
            false,
        );
        manager.register_watch(
            session_id,
            NodePath::parse("/parent").unwrap(),
            true,
        );

        assert_eq!(manager.session_count(), 1);
        assert_eq!(manager.subscription_count(), 2);
    }

    #[test]
    fn test_matching_subscriptions() {
        let mut manager = WatchManager::new();

        let session1 = 1;
        let session2 = 2;

        manager.register_watch(
            session1,
            NodePath::parse("/test").unwrap(),
            false,
        );
        // recursive=true 会自动转换为 /parent/** glob 模式
        manager.register_watch(
            session2,
            NodePath::parse("/parent").unwrap(),
            true,  // recursive=true 自动添加 /** 后缀
        );

        let matching = manager.matching_subscriptions(&NodePath::parse("/test").unwrap());
        assert!(matching.contains(&session1));
        assert!(!matching.contains(&session2));

        let matching = manager.matching_subscriptions(&NodePath::parse("/parent/child").unwrap());
        assert!(matching.contains(&session2));
    }
}