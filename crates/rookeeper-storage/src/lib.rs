use std::path::{Path, PathBuf};

pub const DEFAULT_LOCK_FILE: &str = "rookeeper.lock";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageLayout {
    pub root: PathBuf,
    pub wal_dir: PathBuf,
    pub snapshot_dir: PathBuf,
    pub state_dir: PathBuf,
    pub lock_file: PathBuf,
}

impl StorageLayout {
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            wal_dir: root.join("wal"),
            snapshot_dir: root.join("snapshot"),
            state_dir: root.join("state"),
            lock_file: root.join(DEFAULT_LOCK_FILE),
            root,
        }
    }

    pub fn wal_segment_path(&self, segment_id: u64) -> PathBuf {
        self.wal_dir.join(format!("wal-{segment_id:016}.log"))
    }

    pub fn snapshot_path(&self, generation: u64) -> PathBuf {
        self.snapshot_dir
            .join(format!("snapshot-{generation:016}.bin"))
    }
}

pub fn checksum(bytes: &[u8]) -> u32 {
    crc32fast::hash(bytes)
}

#[cfg(test)]
mod tests {
    use super::{checksum, StorageLayout};

    #[test]
    fn builds_expected_layout() {
        let layout = StorageLayout::from_root("data");
        assert!(layout
            .wal_segment_path(7)
            .ends_with("wal-0000000000000007.log"));
        assert!(layout
            .snapshot_path(3)
            .ends_with("snapshot-0000000000000003.bin"));
    }

    #[test]
    fn checksum_is_stable() {
        assert_eq!(checksum(b"rookeeper"), 2_164_299_473);
    }
}
