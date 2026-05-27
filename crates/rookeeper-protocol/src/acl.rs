//! ACL（访问控制列表）定义 - 权限管理和主题验证
//! 
//! 设计原则：
//! - 权限检查在入口点进行，避免业务逻辑泄露
//! - 支持通配符主题简化管理（`*` 表示任何人）
//! - 权限是累加的，用户拥有所有匹配条目的并集权限

use serde::{Deserialize, Serialize};

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
    pub fn allows(&self, subject: &str, permission: Permission) -> bool {
        self.entries.iter().any(|entry| {
            (entry.subject == "*" || entry.subject == subject)
                && entry.permissions.contains(&permission)
        })
    }
}

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