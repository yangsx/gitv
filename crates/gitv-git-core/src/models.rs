use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Oid([u8; 20]);

impl Oid {
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn short_hex(&self) -> String {
        self.to_hex()[..7].to_string()
    }
}

impl std::fmt::Debug for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oid({})", self.to_hex())
    }
}

impl std::fmt::Display for Oid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub oid: Oid,
    pub short_oid: String,
    pub message: String,
    pub summary: String,
    pub author: Author,
    pub committer: Author,
    pub author_time: DateTime<Utc>,
    pub commit_time: DateTime<Utc>,
    pub parent_oids: Vec<Oid>,
    pub refs: Vec<Ref>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitDetails {
    pub info: CommitInfo,
    pub tree_oid: Oid,
    pub signature: Option<String>,
    pub changed_files: Vec<FileChange>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepositoryInfo {
    pub path: PathBuf,
    pub head_branch: Option<String>,
    pub head_commit: Option<Oid>,
    pub is_bare: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorktreeStatus {
    pub staged_count: usize,
    pub unstaged_count: usize,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Ref {
    Branch(BranchRef),
    Tag(TagRef),
    Remote(RemoteRef),
    Head,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchRef {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagRef {
    pub name: String,
    pub oid: Oid,
    pub annotation: Option<TagAnnotation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagAnnotation {
    pub tagger: Author,
    pub message: String,
    pub time: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteRef {
    pub name: String,
    pub remote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub is_binary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ChangeType {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    SubmoduleUpdated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FileNodeType {
    File,
    Directory,
    Symlink,
    Submodule,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: PathBuf,
    pub node_type: FileNodeType,
    pub children: Vec<FileTreeNode>,
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub oid: Oid,
    pub parent_oid: Oid,
    pub message: String,
    pub author: Author,
    pub time: DateTime<Utc>,
    pub file_summary: Vec<StashFileSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StashFileSummary {
    pub path: PathBuf,
    pub change_type: StashChangeType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum StashChangeType {
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReflogEntry {
    pub oid: Oid,
    pub old_oid: Option<Oid>,
    pub ref_name: String,
    pub message: String,
    pub author: Author,
    pub time: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oid_from_bytes_roundtrip() {
        let bytes: [u8; 20] = [
            0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let oid = Oid::from_bytes(bytes);
        assert_eq!(oid.as_bytes(), &bytes);
    }

    #[test]
    fn oid_to_hex() {
        let bytes: [u8; 20] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10, 0x00, 0x00, 0x00, 0x01,
        ];
        let oid = Oid::from_bytes(bytes);
        assert_eq!(oid.to_hex(), "0123456789abcdeffedcba987654321000000001");
    }

    #[test]
    fn oid_short_hex() {
        let bytes: [u8; 20] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10, 0x00, 0x00, 0x00, 0x01,
        ];
        let oid = Oid::from_bytes(bytes);
        assert_eq!(oid.short_hex(), "0123456");
    }

    #[test]
    fn oid_equality() {
        let a = Oid::from_bytes([0u8; 20]);
        let b = Oid::from_bytes([0u8; 20]);
        let c = Oid::from_bytes([1u8; 20]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn oid_display() {
        let oid = Oid::from_bytes([0u8; 20]);
        assert_eq!(format!("{oid}"), "0000000000000000000000000000000000000000");
    }
}
