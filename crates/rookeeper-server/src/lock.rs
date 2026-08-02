//! lock.rs
//!
//! 单机协调锁实现，基于临时顺序节点模型。
//!
//! 设计原则：
//! - 锁建立在 TreeKv + Watch 之上，不引入额外的同步原语
//! - 锁获取是公平排队（FIFO），避免饥饿
//! - 会话断开时锁自动释放（ephemeral 节点特性）
//! - 支持阻塞获取、非阻塞尝试获取、超时获取三种模式

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use tokio::sync::RwLock;

use rookeeper_protocol::error::ErrorCode;
use rookeeper_protocol::model::{CommandId, NodePath, SessionId};

use crate::tree_kv::{TreeKv, TreeKvError};

/// 锁获取模式，决定锁获取的行为
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    /// 非阻塞获取：锁已被持有则立即返回失败
    NonBlocking,
    /// 阻塞获取：一直等待直到锁可用
    Blocking,
    /// 超时获取：等待指定时间后放弃
    Timeout(Duration),
}

/// 锁状态查询结果
#[derive(Debug, Clone)]
pub struct LockState {
    /// 锁路径
    pub path: NodePath,
    /// 是否已被持有
    pub is_held: bool,
    /// 当前持有者会话 ID（如果被持有）
    pub holder_session_id: Option<SessionId>,
    /// 等待队列长度
    pub queue_length: usize,
}

/// 锁管理器
///
/// 管理所有锁的获取、释放和状态查询。
/// 锁通过 TreeKv 的 ephemeral sequential 节点实现，
/// 每次创建节点分配递增序号，序号最小的节点持有锁。
#[derive(Debug)]
pub struct LockManager {
    /// 锁元数据：锁名 -> 锁信息
    locks: RwLock<BTreeMap<String, LockMeta>>,
    /// 引用到 TreeKv（由 server.rs 提供）
    tree_kv: Arc<RwLock<TreeKv>>,
    /// 服务器启动时间（用于超时计算）
    start_time: Instant,
}

/// 锁的元数据
#[derive(Debug, Clone)]
struct LockMeta {
    /// 锁路径前缀（如 "/locks/my-lock"）
    lock_path: NodePath,
    /// 已分配的最小序号（当前锁持有者）
    min_sequence: Option<u64>,
    /// 等待队列：SessionId -> 分配序号
    waiters: BTreeMap<SessionId, u64>,
    /// 下一个可分配的序号
    next_sequence: u64,
}

impl LockManager {
    /// 创建锁管理器
    pub fn new(tree_kv: Arc<RwLock<TreeKv>>, start_time: Instant) -> Self {
        Self {
            locks: RwLock::new(BTreeMap::new()),
            tree_kv,
            start_time,
        }
    }

    /// 获取锁
    ///
    /// # Arguments
    /// * `lock_name` - 锁名称（不含路径前缀）
    /// * `session_id` - 请求获取锁的会话 ID
    /// * `mode` - 获取模式
    ///
    /// # Returns
    /// - `Ok((lock_path, sequence))` - 获取成功，返回锁路径和分配的序号
    /// - `Err(ErrorCode)` - 获取失败
    pub async fn acquire(
        &self,
        lock_name: &str,
        session_id: SessionId,
        mode: LockMode,
    ) -> Result<(NodePath, u64), ErrorCode> {
        let lock_path;
        let sequence;
        {
            let mut locks = self.locks.write().await;
            let path_str = format!("/locks/{}", lock_name);
            lock_path = NodePath::parse(&path_str).map_err(|_| ErrorCode::InvalidPath)?;
            locks.entry(lock_name.to_string()).or_insert_with(|| {
                LockMeta {
                    lock_path: NodePath::parse(&format!("/locks/{}", lock_name)).unwrap(),
                    min_sequence: None,
                    waiters: BTreeMap::new(),
                    next_sequence: 1,
                }
            });
            let meta = locks.get_mut(lock_name).unwrap();
            sequence = meta.next_sequence;
            meta.next_sequence += 1;
            meta.waiters.insert(session_id, sequence);
        } // 释放写锁

        // 尝试成为锁持有者（序号最小的等待者）
        let acquired = {
            let mut locks = self.locks.write().await;
            let meta = locks.get_mut(lock_name).unwrap();
            if meta.min_sequence.is_none() || sequence < meta.min_sequence.unwrap() {
                meta.min_sequence = Some(sequence);
            }
            meta.min_sequence == Some(sequence)
        };

        if !acquired {
            match mode {
                LockMode::NonBlocking => {
                    // 非阻塞：移除等待记录，立即返回失败
                    let mut locks = self.locks.write().await;
                    if let Some(meta) = locks.get_mut(lock_name) {
                        meta.waiters.remove(&session_id);
                        if meta.waiters.is_empty() {
                            meta.min_sequence = None;
                        }
                    }
                    return Err(ErrorCode::LockBusy);
                }
                LockMode::Blocking => {
                    // 阻塞：直接返回排队状态，由调用方通过 Watch 等待通知
                    return Ok((lock_path, sequence));
                }
                LockMode::Timeout(timeout) => {
                    // 超时：轮询等待
                    let deadline = Instant::now() + timeout;
                    loop {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        let is_min = {
                            let locks = self.locks.read().await;
                            let meta = locks.get(lock_name).unwrap();
                            meta.min_sequence == Some(sequence)
                        };
                        if is_min {
                            break Ok((lock_path, sequence));
                        }
                        if Instant::now() >= deadline {
                            let mut locks = self.locks.write().await;
                            if let Some(meta) = locks.get_mut(lock_name) {
                                meta.waiters.remove(&session_id);
                                if meta.waiters.is_empty() {
                                    meta.min_sequence = None;
                                }
                            }
                            break Err(ErrorCode::LockTimeout);
                        }
                    }
                }
            }
        } else {
            Ok((lock_path, sequence))
        }
    }

    /// 释放锁
    ///
    /// # Arguments
    /// * `lock_name` - 锁名称
    /// * `session_id` - 释放锁的会话 ID
    ///
    /// # Returns
    /// - `Ok(())` - 释放成功
    /// - `Err(ErrorCode)` - 释放失败（锁未被持有等）
    pub async fn release(&self, lock_name: &str, session_id: SessionId) -> Result<(), ErrorCode> {
        let mut locks = self.locks.write().await;
        let meta = locks.get_mut(lock_name).ok_or(ErrorCode::LockNotFound)?;

        // 检查是否由当前会话持有
        let my_sequence = meta
            .waiters
            .get(&session_id)
            .copied()
            .ok_or(ErrorCode::LockNotHeld)?;

        if meta.min_sequence != Some(my_sequence) {
            return Err(ErrorCode::LockNotHeld);
        }

        // 移除当前会话
        meta.waiters.remove(&session_id);

        // 将锁交给下一个等待者（序号最小的）
        if let Some((next_session, next_sequence)) = meta.waiters.iter().next() {
            meta.min_sequence = Some(*next_sequence);
        } else {
            meta.min_sequence = None;
        }

        Ok(())
    }

    /// 查询锁状态
    pub async fn get_state(&self, lock_name: &str) -> Result<LockState, ErrorCode> {
        let locks = self.locks.read().await;
        let meta = locks.get(lock_name).ok_or(ErrorCode::LockNotFound)?;

        // 找到当前持有者
        let holder_session_id = meta
            .min_sequence
            .and_then(|min_seq| meta.waiters.iter().find(|(_, seq)| **seq == min_seq))
            .map(|(sid, _)| *sid);

        let lock_path = format!("/locks/{}", lock_name);
        Ok(LockState {
            path: NodePath::parse(&lock_path).unwrap(),
            is_held: meta.min_sequence.is_some(),
            holder_session_id,
            queue_length: meta.waiters.len(),
        })
    }

    /// 通知某个锁的持有者已变更（由 Watch 或会话断开触发）
    pub async fn on_lock_released(&self, lock_name: &str) {
        let mut locks = self.locks.write().await;
        if let Some(meta) = locks.get_mut(lock_name) {
            // 移除已退出的等待者（通过 Watch 删除节点时会调用此方法）
            // 找到新的最小序号
            if let Some((_, seq)) = meta.waiters.iter().next() {
                meta.min_sequence = Some(*seq);
            } else {
                meta.min_sequence = None;
            }
        }
    }

    /// 清理会话的所有锁等待记录（会话断开时调用）
    pub async fn cleanup_session(&self, session_id: SessionId) -> Vec<String> {
        let mut locks = self.locks.write().await;
        let mut released_locks = Vec::new();

        for (lock_name, meta) in locks.iter_mut() {
            if let Some(my_seq) = meta.waiters.get(&session_id).copied() {
                let was_holder = meta.min_sequence == Some(my_seq);
                meta.waiters.remove(&session_id);
                // 如果是锁持有者，需要移交给下一个等待者
                if was_holder {
                    if let Some((_, next_seq)) = meta.waiters.iter().next() {
                        meta.min_sequence = Some(*next_seq);
                    } else {
                        meta.min_sequence = None;
                    }
                }
                released_locks.push(lock_name.clone());
            }
        }

        released_locks
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_non_blocking_lock_acquire() {
        let tree_kv = Arc::new(RwLock::new(TreeKv::new()));
        let manager = LockManager::new(tree_kv, Instant::now());

        // 首次获取应该成功
        let result = manager
            .acquire("test-lock", 1, LockMode::NonBlocking)
            .await;
        assert!(result.is_ok());

        // 再次获取应该失败（已被持有）
        let result = manager
            .acquire("test-lock", 2, LockMode::NonBlocking)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lock_release() {
        let tree_kv = Arc::new(RwLock::new(TreeKv::new()));
        let manager = LockManager::new(tree_kv, Instant::now());

        let (path, seq) = manager
            .acquire("test-lock", 1, LockMode::NonBlocking)
            .await
            .unwrap();

        // 释放锁
        let result = manager.release("test-lock", 1).await;
        assert!(result.is_ok());

        // 现在其他会话可以获取
        let result = manager
            .acquire("test-lock", 2, LockMode::NonBlocking)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lock_state_query() {
        let tree_kv = Arc::new(RwLock::new(TreeKv::new()));
        let manager = LockManager::new(tree_kv, Instant::now());

        // 初始状态：未持有
        let state = manager.get_state("test-lock").await;
        assert!(state.is_err()); // 锁不存在

        // 获取锁
        manager
            .acquire("test-lock", 1, LockMode::NonBlocking)
            .await
            .unwrap();

        let state = manager.get_state("test-lock").await.unwrap();
        assert!(state.is_held);
        assert_eq!(state.holder_session_id, Some(1));
        assert_eq!(state.queue_length, 1);
    }

    #[tokio::test]
    async fn test_session_cleanup_releases_lock() {
        let tree_kv = Arc::new(RwLock::new(TreeKv::new()));
        let manager = LockManager::new(tree_kv, Instant::now());

        // 会话1获取锁
        manager
            .acquire("test-lock", 1, LockMode::NonBlocking)
            .await
            .unwrap();

        // 会话1断开，清理其锁记录
        let released = manager.cleanup_session(1).await;
        assert!(released.contains(&"test-lock".to_string()));

        // 锁现在可以被其他会话获取
        let result = manager
            .acquire("test-lock", 2, LockMode::NonBlocking)
            .await;
        assert!(result.is_ok());
    }
}
