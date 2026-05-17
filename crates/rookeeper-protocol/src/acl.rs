use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Read,
    Write,
    Create,
    Delete,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AclEntry {
    pub subject: String,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Acl {
    pub entries: Vec<AclEntry>,
}

impl Acl {
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
