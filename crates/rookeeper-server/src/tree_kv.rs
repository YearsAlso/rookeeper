//! tree_kv.rs
//!
//! 树形键值存储实现，提供节点创建、读取、更新、删除和列举功能。
//!
//! 设计原则：
//! - 使用 BTreeMap 存储节点，确保确定性迭代顺序
//! - 版本控制使用命令 ID（而非时间戳）确保状态机确定性
//! - 所有路径操作通过 NodePath::parse 验证

use std::collections::BTreeMap;

use bytes::Bytes;
use rookeeper_protocol::acl::{Acl, AclChecker, Permission};
use rookeeper_protocol::error::ErrorCode;
use rookeeper_protocol::model::{CommandId, NodeKind, NodeMetadata, NodePath};

/// 树形 KV 存储错误类型
#[derive(Debug)]
pub enum TreeKvError {
    /// 路径不存在
    PathNotFound,
    /// 路径已存在
    PathAlreadyExists,
    /// 版本冲突
    VersionConflict,
    /// 权限不足
    PermissionDenied,
    /// 路径无效
    InvalidPath,
    /// 内部错误
    Internal(String),
}

impl From<TreeKvError> for ErrorCode {
    fn from(err: TreeKvError) -> Self {
        match err {
            TreeKvError::PathNotFound => ErrorCode::PathNotFound,
            TreeKvError::PathAlreadyExists => ErrorCode::PathAlreadyExists,
            TreeKvError::VersionConflict => ErrorCode::VersionConflict,
            TreeKvError::PermissionDenied => ErrorCode::PermissionDenied,
            TreeKvError::InvalidPath => ErrorCode::InvalidPath,
            TreeKvError::Internal(_) => ErrorCode::Internal,
        }
    }
}

/// 树形 KV 节点
///
/// 包含节点数据、元数据和访问控制列表
#[derive(Debug, Clone)]
pub struct KvNode {
    /// 节点数据内容
    pub value: Bytes,
    /// 节点元数据
    pub metadata: NodeMetadata,
    /// 访问控制列表
    pub acl: Acl,
}

/// 树形键值存储
///
/// 使用 BTreeMap 存储所有节点，确保确定性迭代顺序
/// 键为规范化后的路径，值为节点数据
#[derive(Debug, Clone)]
pub struct TreeKv {
    /// 节点存储
    nodes: BTreeMap<NodePath, KvNode>,
    /// ACL 检查器
    acl_checker: AclChecker,
    /// 当前命令 ID（用于版本控制）
    command_id: CommandId,
}

impl Default for TreeKv {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeKv {
    /// 创建新的树形 KV 存储
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            acl_checker: AclChecker::new(),
            command_id: 0,
        }
    }

    /// 创建带 ACL 检查器的树形 KV 存储
    pub fn with_acl_checker(acl_checker: AclChecker) -> Self {
        Self {
            nodes: BTreeMap::new(),
            acl_checker,
            command_id: 0,
        }
    }

    /// 获取下一个命令 ID
    fn next_command_id(&mut self) -> CommandId {
        self.command_id += 1;
        self.command_id
    }

    /// 检查权限
    fn check_permission(
        &self,
        path: &NodePath,
        permission: Permission,
        session_subject: &str,
    ) -> Result<(), TreeKvError> {
        // 获取节点的 ACL（如果存在）
        if let Some(node) = self.nodes.get(path) {
            // 如果节点的 ACL 中有匹配条目或通配符，则检查权限
            if node.acl.allows(session_subject, permission) {
                return Ok(());
            }
            // 否则检查 AclChecker 的默认权限
            self.acl_checker
                .check(&node.acl, session_subject, permission)
                .map_err(|_| TreeKvError::PermissionDenied)?;
        }
        Ok(())
    }

    /// 检查父路径是否存在
    #[allow(dead_code)]
    fn parent_exists(&self, path: &NodePath) -> bool {
        let path_str = path.as_str();
        if path_str == "/" {
            return true;
        }

        // 检查所有可能的父路径
        let segments: Vec<&str> = path_str.trim_start_matches('/').split('/').collect();
        for i in 1..segments.len() {
            let parent_path = format!("/{}", segments[..i].join("/"));
            if let Ok(parsed) = NodePath::parse(&parent_path) {
                if self.nodes.contains_key(&parsed) {
                    return true;
                }
            }
        }
        true // 根节点不需要父节点存在
    }

    /// 获取节点的子路径列表
    pub fn list_children(&self, path: &NodePath) -> Vec<NodePath> {
        let prefix = format!("{}/", path.as_str().trim_end_matches('/'));
        self.nodes
            .keys()
            .filter(|p| p.as_str().starts_with(&prefix) && p.as_str() != path.as_str())
            .filter_map(|p| {
                let remainder = p.as_str().strip_prefix(&prefix).unwrap_or("");
                // 只取直接子节点（一级深度）
                if !remainder.contains('/') {
                    Some(p.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// 创建节点
    ///
    /// # Arguments
    /// * `path` - 节点路径
    /// * `value` - 节点值
    /// * `session_subject` - 会话主题（用于 ACL 检查）
    ///
    /// # Returns
    /// - `Ok(())` - 创建成功
    /// - `Err(TreeKvError)` - 创建失败
    pub fn create(
        &mut self,
        path: &NodePath,
        value: Bytes,
        session_subject: &str,
    ) -> Result<(), TreeKvError> {
        // 检查路径是否已存在
        if self.nodes.contains_key(path) {
            return Err(TreeKvError::PathAlreadyExists);
        }

        // 检查权限（Create 权限）
        self.check_permission(path, Permission::Create, session_subject)?;

        let command_id = self.next_command_id();

        let node = KvNode {
            value,
            metadata: NodeMetadata {
                version: 1,
                create_command_id: command_id,
                modify_command_id: command_id,
                kind: NodeKind::Persistent,
            },
            acl: Acl::default(), // 默认 ACL
        };

        self.nodes.insert(path.clone(), node);
        Ok(())
    }

    /// 读取节点
    ///
    /// # Arguments
    /// * `path` - 节点路径
    /// * `session_subject` - 会话主题（用于 ACL 检查）
    ///
    /// # Returns
    /// - `Ok(&KvNode)` - 读取成功
    /// - `Err(TreeKvError)` - 读取失败
    pub fn get(&self, path: &NodePath, session_subject: &str) -> Result<&KvNode, TreeKvError> {
        // 检查权限（Read 权限）
        self.check_permission(path, Permission::Read, session_subject)?;

        self.nodes.get(path).ok_or(TreeKvError::PathNotFound)
    }

    /// 更新节点
    ///
    /// # Arguments
    /// * `path` - 节点路径
    /// * `value` - 新的节点值
    /// * `expected_version` - 期望的版本号（用于乐观锁），None 表示不检查
    /// * `session_subject` - 会话主题（用于 ACL 检查）
    ///
    /// # Returns
    /// - `Ok(())` - 更新成功
    /// - `Err(TreeKvError)` - 更新失败
    pub fn set(
        &mut self,
        path: &NodePath,
        value: Bytes,
        expected_version: Option<u64>,
        session_subject: &str,
    ) -> Result<(), TreeKvError> {
        // 检查权限（Write 权限）
        self.check_permission(path, Permission::Write, session_subject)?;

        let command_id = self.next_command_id();

        let node = self.nodes.get_mut(path).ok_or(TreeKvError::PathNotFound)?;
        if let Some(expected) = expected_version {
            if node.metadata.version != expected {
                return Err(TreeKvError::VersionConflict);
            }
        }

        node.value = value;
        node.metadata.version += 1;
        node.metadata.modify_command_id = command_id;

        Ok(())
    }

    /// 删除节点
    ///
    /// # Arguments
    /// * `path` - 节点路径
    /// * `session_subject` - 会话主题（用于 ACL 检查）
    ///
    /// # Returns
    /// - `Ok(())` - 删除成功
    /// - `Err(TreeKvError)` - 删除失败
    pub fn delete(&mut self, path: &NodePath, session_subject: &str) -> Result<(), TreeKvError> {
        // 检查权限（Delete 权限）
        self.check_permission(path, Permission::Delete, session_subject)?;

        if self.nodes.remove(path).is_none() {
            return Err(TreeKvError::PathNotFound);
        }

        Ok(())
    }

    /// 列举子节点
    ///
    /// # Arguments
    /// * `path` - 父节点路径
    /// * `session_subject` - 会话主题（用于 ACL 检查）
    ///
    /// # Returns
    /// - `Ok(Vec<NodePath>)` - 子节点路径列表
    /// - `Err(TreeKvError)` - 列举失败
    pub fn list(
        &self,
        path: &NodePath,
        session_subject: &str,
    ) -> Result<Vec<NodePath>, TreeKvError> {
        // 检查权限（Read 权限）
        self.check_permission(path, Permission::Read, session_subject)?;

        // 检查路径是否存在
        if !self.nodes.contains_key(path) {
            return Err(TreeKvError::PathNotFound);
        }

        Ok(self.list_children(path))
    }

    /// 获取节点数量
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 获取当前命令 ID
    pub fn command_id(&self) -> CommandId {
        self.command_id
    }

    /// 从快照恢复
    pub fn restore_from_snapshot(&mut self, snapshot: &TreeKvSnapshot) {
        self.nodes.clear();
        for (path, node_data) in &snapshot.nodes {
            if let Ok(parsed_path) = NodePath::parse(path) {
                self.nodes.insert(
                    parsed_path,
                    KvNode {
                        value: Bytes::from(node_data.value.clone()),
                        metadata: NodeMetadata {
                            version: node_data.version,
                            create_command_id: node_data.create_command_id,
                            modify_command_id: node_data.modify_command_id,
                            kind: node_data.kind,
                        },
                        acl: Acl::default(),
                    },
                );
            }
        }
        self.command_id = snapshot.max_command_id;
    }
}

/// 快照节点数据（用于序列化）
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotNodeData {
    pub value: Vec<u8>,
    pub version: u64,
    pub create_command_id: CommandId,
    pub modify_command_id: CommandId,
    pub kind: NodeKind,
}

/// 树形 KV 快照
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct TreeKvSnapshot {
    pub nodes: BTreeMap<String, SnapshotNodeData>,
    pub max_command_id: CommandId,
}

impl TreeKv {
    /// 创建快照
    pub fn create_snapshot(&self) -> TreeKvSnapshot {
        let mut nodes = BTreeMap::new();
        for (path, node) in &self.nodes {
            nodes.insert(
                path.as_str().to_string(),
                SnapshotNodeData {
                    value: node.value.to_vec(),
                    version: node.metadata.version,
                    create_command_id: node.metadata.create_command_id,
                    modify_command_id: node.metadata.modify_command_id,
                    kind: node.metadata.kind,
                },
            );
        }
        TreeKvSnapshot {
            nodes,
            max_command_id: self.command_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get() {
        let mut tree = TreeKv::new();
        let path = NodePath::parse("/test/node").unwrap();

        tree.create(&path, Bytes::from("value"), "user").unwrap();

        let node = tree.get(&path, "user").unwrap();
        assert_eq!(node.value.as_ref(), b"value");
        assert_eq!(node.metadata.version, 1);
    }

    #[test]
    fn test_version_conflict() {
        let mut tree = TreeKv::new();
        let path = NodePath::parse("/test/node").unwrap();

        tree.create(&path, Bytes::from("value"), "user").unwrap();

        // 错误的版本号应该失败
        let result = tree.set(&path, Bytes::from("new"), Some(999), "user");
        assert!(matches!(result, Err(TreeKvError::VersionConflict)));

        // 正确的版本号应该成功
        tree.set(&path, Bytes::from("new"), Some(1), "user")
            .unwrap();
        let node = tree.get(&path, "user").unwrap();
        assert_eq!(node.value.as_ref(), b"new");
        assert_eq!(node.metadata.version, 2);
    }

    #[test]
    fn test_delete() {
        let mut tree = TreeKv::new();
        let path = NodePath::parse("/test/node").unwrap();

        tree.create(&path, Bytes::from("value"), "user").unwrap();
        tree.delete(&path, "user").unwrap();

        let result = tree.get(&path, "user");
        assert!(matches!(result, Err(TreeKvError::PathNotFound)));
    }

    #[test]
    fn test_list_children() {
        let mut tree = TreeKv::new();

        tree.create(
            &NodePath::parse("/parent/child1").unwrap(),
            Bytes::from("v1"),
            "user",
        )
        .unwrap();
        tree.create(
            &NodePath::parse("/parent/child2").unwrap(),
            Bytes::from("v2"),
            "user",
        )
        .unwrap();
        tree.create(
            &NodePath::parse("/parent/child3").unwrap(),
            Bytes::from("v3"),
            "user",
        )
        .unwrap();
        tree.create(
            &NodePath::parse("/other").unwrap(),
            Bytes::from("v"),
            "user",
        )
        .unwrap();

        let children = tree.list_children(&NodePath::parse("/parent").unwrap());
        assert_eq!(children.len(), 3);

        let child_paths: Vec<_> = children.iter().map(|p| p.as_str().to_string()).collect();
        assert!(child_paths.contains(&"/parent/child1".to_string()));
        assert!(child_paths.contains(&"/parent/child2".to_string()));
        assert!(child_paths.contains(&"/parent/child3".to_string()));
    }

    #[test]
    fn test_snapshot_and_restore() {
        let mut tree = TreeKv::new();
        tree.create(
            &NodePath::parse("/test/node1").unwrap(),
            Bytes::from("v1"),
            "user",
        )
        .unwrap();
        tree.create(
            &NodePath::parse("/test/node2").unwrap(),
            Bytes::from("v2"),
            "user",
        )
        .unwrap();

        let snapshot = tree.create_snapshot();

        let mut restored = TreeKv::new();
        restored.restore_from_snapshot(&snapshot);

        assert_eq!(restored.len(), 2);
        assert_eq!(
            restored
                .get(&NodePath::parse("/test/node1").unwrap(), "user")
                .unwrap()
                .value
                .as_ref(),
            b"v1"
        );
    }
}
