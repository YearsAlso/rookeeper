//! ACL（访问控制列表）定义 - 权限管理和主题验证
//!
//! 设计原则：
//! - 权限检查在入口点进行，避免业务逻辑泄露
//! - 支持通配符主题简化管理（`*` 表示任何人）
//! - 权限是累加的，用户拥有所有匹配条目的并集权限

use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;

/// 操作权限类型，定义可以对节点执行的动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// 读取节点数据和元数据
    Read,
    /// 更新现有节点内容
    Write,
    /// 创建新节点或子节点
    Create,
    /// 删除节点
    Delete,
    /// 修改 ACL 本身（需要最高权限）
    Admin,
}

/// 单个 ACL 条目，定义某个主题的权限集合
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AclEntry {
    /// 主题标识，通常是用户名或服务名
    /// `*` 表示匹配所有主题（公共权限）
    pub subject: String,
    /// 该主题拥有的权限列表
    pub permissions: Vec<Permission>,
}

/// 访问控制列表，包含多条规则
///
/// 权限检查逻辑：
/// 1. 查找匹配主题的条目（支持 `*` 通配符）
/// 2. 如果任何匹配条目包含请求的权限，则允许
/// 3. 否则拒绝访问
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Acl {
    pub entries: Vec<AclEntry>,
}

impl Acl {
    /// 检查指定主题是否拥有特定权限
    ///
    /// 通配符 `*` 匹配所有主题，用于定义公共访问权限
    ///
    /// 空 ACL（无条目）默认允许所有操作，这是 Phase 1 的简化策略
    pub fn allows(&self, subject: &str, permission: Permission) -> bool {
        // Phase 1 简化：空 ACL 允许所有操作
        if self.entries.is_empty() {
            return true;
        }
        self.entries.iter().any(|entry| {
            (entry.subject == "*" || entry.subject == subject)
                && entry.permissions.contains(&permission)
        })
    }
}

/// ACL 检查器
///
/// 用于在操作前验证会话是否具有足够权限
#[derive(Debug, Clone, Default)]
pub struct AclChecker {
    /// 默认权限（当没有 ACL 条目时的兜底策略）
    default_permissions: Vec<Permission>,
}

impl AclChecker {
    /// 创建新的 ACL 检查器
    pub fn new() -> Self {
        Self {
            default_permissions: Vec::new(),
        }
    }

    /// 设置默认权限
    pub fn with_default_permissions(mut self, permissions: Vec<Permission>) -> Self {
        self.default_permissions = permissions;
        self
    }

    /// 检查会话是否对指定路径拥有特定权限
    ///
    /// # Arguments
    /// * `acl` - 节点的访问控制列表
    /// * `session_subject` - 会话的主题标识（通常是用户名）
    /// * `permission` - 需要检查的权限
    ///
    /// # Returns
    /// - `Ok(())` - 权限足够
    /// - `Err(ErrorCode::PermissionDenied)` - 权限不足
    pub fn check(
        &self,
        acl: &Acl,
        session_subject: &str,
        permission: Permission,
    ) -> Result<(), ErrorCode> {
        // 首先检查 ACL 条目
        if acl.allows(session_subject, permission) {
            return Ok(());
        }

        // 如果没有匹配，检查默认权限
        if self.default_permissions.contains(&permission) {
            return Ok(());
        }

        Err(ErrorCode::PermissionDenied)
    }
}

/// Session 元数据，用于 ACL 检查时获取主题标识
///
/// 在实际实现中，这些数据可能存储在 Session 管理器中
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionMeta {
    /// 会话唯一标识
    pub session_id: u64,
    /// 会话主题（通常是用户名或服务名）
    pub subject: String,
}

impl SessionMeta {
    /// 创建新的会话元数据
    pub fn new(session_id: u64, subject: impl Into<String>) -> Self {
        Self {
            session_id,
            subject: subject.into(),
        }
    }
}

// 重新导出 ErrorCode 以便在 acl 模块使用

#[cfg(test)]
mod tests {
    use super::{Acl, AclEntry, Permission};

    #[test]
    fn wildcard_subject_matches() {
        let acl = Acl {
            entries: vec![AclEntry {
                subject: "*".to_string(),
                permissions: vec![Permission::Read],
            }],
        };

        assert!(acl.allows("operator", Permission::Read));
        assert!(!acl.allows("operator", Permission::Write));
    }
}
