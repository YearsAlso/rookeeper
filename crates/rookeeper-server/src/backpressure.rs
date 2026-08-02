//! backpressure.rs
//!
//! Watch 事件分发背压模块。
//!
//! 设计原则：
//! - 每个会话维护独立的分发队列（bounded channel），避免慢消费者阻塞快生产者
//! - Drop-Newest 策略：队列满时丢弃新事件，优先保证旧事件到达
//! - 事件顺序由 TreeKv 命令 ID 保证单调递增
//! - 提供 delivery queue 统计（队列深度、丢弃计数）用于监控

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

/// 分发队列统计
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub capacity: usize,
    pub current_depth: usize,
    pub dropped_total: u64,
}

/// 分发管理器
///
/// 为每个会话提供独立的 bounded delivery queue，
/// 应用 Drop-Newest 背压策略：当队列满时丢弃新事件而非阻塞写路径。
///
/// 使用方式：
/// 1. 服务端接受连接时，调用 `register_session` 获取 receiver
/// 2. 在接收帧的循环中，从 receiver 读取事件并发送给客户端
/// 3. 连接断开时，调用 `unregister_session`
/// 4. 事件触发时，调用 `deliver` 或 `deliver_batch`
///
/// 注意：Phase 1 的 broadcast 通道仍然保留用于全局事件分发。
/// 此模块作为 Phase 2 增强，处理需要背压控制的场景。
#[derive(Debug)]
pub struct DeliveryManager {
    /// 会话 ID → 发送端
    senders: HashMap<u64, mpsc::Sender<crate::watch::WatchEvent>>,
    /// 会话 ID → 丢弃计数
    dropped: HashMap<u64, AtomicU64>,
    /// 默认队列容量
    default_capacity: usize,
}

impl DeliveryManager {
    /// 创建分发管理器
    ///
    /// `default_capacity` 是每个会话队列的默认容量。
    /// 当队列满时采用 Drop-Newest 策略。
    pub fn new(default_capacity: usize) -> Self {
        Self {
            senders: HashMap::new(),
            dropped: HashMap::new(),
            default_capacity,
        }
    }

    /// 为会话注册分发队列
    ///
    /// 返回 receiver 供连接处理器读取事件。
    /// 如果队列已存在则返回 None。
    pub fn register_session(&mut self, session_id: u64) -> Option<mpsc::Receiver<crate::watch::WatchEvent>> {
        if self.senders.contains_key(&session_id) {
            return None;
        }
        let (tx, rx) = mpsc::channel(self.default_capacity);
        self.senders.insert(session_id, tx);
        self.dropped.insert(session_id, AtomicU64::new(0));
        Some(rx)
    }

    /// 注销会话，清理其分发队列
    pub fn unregister_session(&mut self, session_id: u64) {
        self.senders.remove(&session_id);
        self.dropped.remove(&session_id);
    }

    /// 向指定会话发送事件（带背压控制）
    ///
    /// 返回 true 表示发送成功，false 表示队列满（事件被丢弃）。
    /// 当队列已满时采用 Drop-Newest 策略：新事件被丢弃，旧事件得以保留。
    pub fn deliver(&self, session_id: u64, event: crate::watch::WatchEvent) -> bool {
        if let Some(tx) = self.senders.get(&session_id) {
            match tx.try_send(event) {
                Ok(()) => true,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    if let Some(c) = self.dropped.get(&session_id) {
                        c.fetch_add(1, Ordering::Relaxed);
                    }
                    false
                }
                Err(mpsc::error::TrySendError::Closed(_)) => false,
            }
        } else {
            false // 会话不存在，静默丢弃
        }
    }

    /// 广播事件到多个会话
    ///
    /// 返回每个会话的发送结果（成功/丢弃）。
    pub fn deliver_batch(
        &self,
        session_ids: &[u64],
        event: crate::watch::WatchEvent,
    ) -> HashMap<u64, bool> {
        let mut results = HashMap::new();
        for &sid in session_ids {
            let success = self.deliver(sid, event.clone());
            results.insert(sid, success);
        }
        results
    }

    /// 获取指定会话的队列统计
    pub fn stats(&self, session_id: u64) -> Option<QueueStats> {
        let tx = self.senders.get(&session_id)?;
        let dropped = self.dropped.get(&session_id)?;
        let capacity = self.default_capacity;
        // mpsc::Sender 没有 max_capacity API，只能用配置的 default_capacity
        Some(QueueStats {
            capacity,
            current_depth: 0, // try_send 不暴露深度
            dropped_total: dropped.load(Ordering::Relaxed),
        })
    }

    /// 获取所有会话的队列统计摘要
    pub fn all_stats(&self) -> HashMap<u64, QueueStats> {
        self.senders
            .keys()
            .filter_map(|&sid| self.stats(sid).map(|s| (sid, s)))
            .collect()
    }

    /// 获取总丢弃计数（所有会话）
    pub fn total_dropped(&self) -> u64 {
        self.dropped.values().map(|c| c.load(Ordering::Relaxed)).sum()
    }

    /// 活跃会话数
    pub fn active_session_count(&self) -> usize {
        self.senders.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_unregister_session() {
        let mut dm = DeliveryManager::new(10);
        let rx = dm.register_session(1);
        assert!(rx.is_some());
        assert!(dm.register_session(1).is_none()); // 重复注册

        dm.unregister_session(1);
        assert!(dm.stats(1).is_none());
    }

    #[tokio::test]
    async fn test_drop_newest_when_full() {
        let mut dm = DeliveryManager::new(3); // 容量 3
        let _rx = dm.register_session(1);

        let event = crate::watch::WatchEvent {
            kind: crate::watch::WatchEventKind::NodeCreated,
            path: rookeeper_protocol::model::NodePath::parse("/test").unwrap(),
            session_id: 1,
        };

        // 填满队列
        for i in 0..3 {
            let ev = crate::watch::WatchEvent {
                kind: crate::watch::WatchEventKind::NodeCreated,
                path: rookeeper_protocol::model::NodePath::parse(&format!("/node{}", i)).unwrap(),
                session_id: 1,
            };
            assert!(dm.deliver(1, ev));
        }

        // 队列已满，新事件被丢弃
        assert!(!dm.deliver(1, event));
        assert!(dm.stats(1).unwrap().dropped_total >= 1);
    }

    #[tokio::test]
    async fn test_nonexistent_session_returns_false() {
        let dm = DeliveryManager::new(10);
        let event = crate::watch::WatchEvent {
            kind: crate::watch::WatchEventKind::NodeCreated,
            path: rookeeper_protocol::model::NodePath::parse("/test").unwrap(),
            session_id: 1,
        };
        assert!(!dm.deliver(999, event));
    }

    #[tokio::test]
    async fn test_batch_deliver() {
        let mut dm = DeliveryManager::new(10);
        let _rx1 = dm.register_session(1);
        let _rx2 = dm.register_session(2);

        let event = crate::watch::WatchEvent {
            kind: crate::watch::WatchEventKind::NodeCreated,
            path: rookeeper_protocol::model::NodePath::parse("/event").unwrap(),
            session_id: 1,
        };

        let results = dm.deliver_batch(&[1, 2], event);
        assert!(results[&1]);
        assert!(results[&2]);
    }

    #[tokio::test]
    async fn test_total_dropped() {
        let mut dm = DeliveryManager::new(2);
        let _rx = dm.register_session(1);

        let event = crate::watch::WatchEvent {
            kind: crate::watch::WatchEventKind::NodeCreated,
            path: rookeeper_protocol::model::NodePath::parse("/x").unwrap(),
            session_id: 1,
        };

        for _ in 0..5 {
            dm.deliver(1, event.clone());
        }

        assert!(dm.total_dropped() >= 3);
    }
}
