//! 模型定义 - 节点路径、会话、节点元数据等核心数据结构
//!
//! 设计原则：
//! - 路径规范化集中在此处，避免散落在业务代码中处理反斜杠等兼容性问题
//! - 确保工业控制环境下的路径安全性（禁止父路径遍历）

use std::fmt::{Display, Formatter};

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::acl::Acl;

/// 命令 ID，用于追踪操作序列和幂等性
pub type CommandId = u64;

/// 会话 ID，用于标识客户端连接生命周期
pub type SessionId = u64;

/// 节点路径，经过规范化后统一使用 Unix 风格 `/` 分隔符
///
/// 规范化规则：
/// - 输入允许 `\` 和 `/` 混合（兼容 Windows 和工业环境）
/// - 去除重复分隔符（如 `//`）
/// - 跳过空段和 `.`（当前目录引用）
/// - 拒绝 `..`（防止父路径遍历攻击）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodePath(String);

impl NodePath {
    /// 解析并规范化路径字符串
    ///
    /// 失败场景：路径包含 `..` 父路径遍历时返回错误
    /// 这是安全考量，防止客户端通过 `..` 访问受保护目录
    pub fn parse(raw: &str) -> Result<Self, &'static str> {
        // 统一转换为正斜杠，兼容 Windows 路径格式
        let replaced = raw.replace('\\', "/");
        let mut segments = Vec::new();

        for segment in replaced.split('/') {
            // 跳过空段（处理重复分隔符）和当前目录引用
            if segment.is_empty() || segment == "." {
                continue;
            }
            // 拒绝父路径遍历，防止路径逃逸到受保护区域
            if segment == ".." {
                return Err("parent path segments are not allowed");
            }
            segments.push(segment);
        }

        // 空路径解析为根路径，保持一致性
        let normalized = if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments.join("/"))
        };

        Ok(Self(normalized))
    }

    /// 获取规范化后的路径字符串引用
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NodePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 节点类型，决定节点的生命周期和访问语义
///
/// 工业控制场景需要区分持久节点（重启后保留）和临时节点（会话结束时自动清理）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// 持久节点，重启后依然存在，需要显式删除
    Persistent,
    /// 临时节点，所属会话断开时自动删除
    Ephemeral,
    /// 顺序节点，每次创建时自动分配递增序号，用于 FIFO 队列等场景
    Sequential,
    /// 临时顺序节点，结合两者特性
    EphemeralSequential,
}

/// 节点的元数据信息，用于版本控制和操作追踪
///
/// 设计理由：使用命令 ID 而非时间戳，确保状态机的确定性
/// 时间戳在分布式环境中会引入不一致性
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// 节点内容版本，每次修改递增，用于乐观锁检测
    pub version: u64,
    /// 创建此节点的命令 ID，用于审计和血缘追踪
    pub create_command_id: CommandId,
    /// 最后修改此节点的命令 ID
    pub modify_command_id: CommandId,
    /// 节点类型
    pub kind: NodeKind,
}

/// 完整的节点记录，包含数据、元数据和访问控制
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRecord {
    /// 节点路径
    pub path: NodePath,
    /// 节点数据内容，二进制格式支持任意数据类型
    pub value: Bytes,
    /// 节点元数据
    pub metadata: NodeMetadata,
    /// 访问控制列表
    pub acl: Acl,
}

/// 会话租约，用于管理客户端连接的存活状态
///
/// 设计考量：租约超时机制比心跳更高效，减少网络往返
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLease {
    /// 会话唯一标识
    pub session_id: SessionId,
    /// 租约超时时间（毫秒），客户端需要在此期间续约
    pub timeout_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::NodePath;

    #[test]
    fn normalizes_backslashes_and_duplicate_separators() {
        let path = NodePath::parse("\\plant\\\\line1/robot3//config").unwrap();
        assert_eq!(path.as_str(), "/plant/line1/robot3/config");
    }

    #[test]
    fn rejects_parent_segments() {
        assert!(NodePath::parse("/plant/../config").is_err());
    }
}
