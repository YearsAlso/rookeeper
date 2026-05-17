use std::fmt::{Display, Formatter};

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::acl::Acl;

pub type CommandId = u64;
pub type SessionId = u64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodePath(String);

impl NodePath {
    pub fn parse(raw: &str) -> Result<Self, &'static str> {
        let replaced = raw.replace('\\', "/");
        let mut segments = Vec::new();

        for segment in replaced.split('/') {
            if segment.is_empty() || segment == "." {
                continue;
            }
            if segment == ".." {
                return Err("parent path segments are not allowed");
            }
            segments.push(segment);
        }

        let normalized = if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments.join("/"))
        };

        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for NodePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Persistent,
    Ephemeral,
    Sequential,
    EphemeralSequential,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub version: u64,
    pub create_command_id: CommandId,
    pub modify_command_id: CommandId,
    pub kind: NodeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub path: NodePath,
    pub value: Bytes,
    pub metadata: NodeMetadata,
    pub acl: Acl,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLease {
    pub session_id: SessionId,
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
