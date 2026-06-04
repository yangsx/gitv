use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::error::OidError;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Oid([u8; 20]);

impl Serialize for Oid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Oid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = String::deserialize(deserializer)?;
        Oid::from_hex(&hex).map_err(serde::de::Error::custom)
    }
}

impl Oid {
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    pub fn from_hex(s: &str) -> Result<Self, OidError> {
        if s.len() != 40 {
            return Err(OidError::InvalidLength(s.len()));
        }
        let s_bytes = s.as_bytes();
        let mut bytes = [0u8; 20];
        for (i, chunk) in s_bytes.chunks_exact(2).enumerate() {
            let hi = hex_val(chunk[0], i * 2)?;
            let lo = hex_val(chunk[1], i * 2 + 1)?;
            bytes[i] = (hi << 4) | lo;
        }
        Ok(Self(bytes))
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn short_hex(&self) -> String {
        self.to_hex()[..7].to_string()
    }
}

fn hex_val(c: u8, pos: usize) -> Result<u8, OidError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(OidError::InvalidChar(pos)),
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

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum DiffMode {
    #[default]
    Normal,
    WordDiff,
    StatOnly,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub enum WhitespaceMode {
    #[default]
    None,
    IgnoreSpaceChange,
    IgnoreAllSpace,
    IgnoreBlankLines,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiffSummary {
    pub files: Vec<FileDiffSummary>,
    pub stats: DiffStats,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDiffSummary {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
    pub is_binary: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub hunks: Vec<Hunk>,
    pub is_binary: bool,
    pub old_size: Option<u64>,
    pub new_size: Option<u64>,
    pub truncated_at: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Hunk {
    pub old_start: usize,
    pub old_count: usize,
    pub new_start: usize,
    pub new_count: usize,
    pub lines: Vec<DiffLine>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum DiffLine {
    Context {
        content: String,
        old_line: usize,
        new_line: usize,
    },
    Addition {
        content: String,
        new_line: usize,
    },
    Deletion {
        content: String,
        old_line: usize,
    },
    WordDiff {
        content: String,
        old_line: usize,
        new_line: usize,
        segments: Vec<WordDiffSegment>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct WordDiffSegment {
    pub text: String,
    pub kind: WordDiffKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WordDiffKind {
    Unchanged,
    Added,
    Removed,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
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
pub struct FileHistoryEntry {
    pub commit_oid: Oid,
    pub path: PathBuf,
    pub old_path: Option<PathBuf>,
    pub summary: String,
    pub author: Author,
    pub time: DateTime<Utc>,
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecentRepository {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitBatch {
    pub commits: Vec<CommitInfo>,
    pub batch_index: usize,
    pub has_more: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CommitFilter {
    pub refs: Option<Vec<String>>,
    pub date_range: Option<DateRange>,
    pub author: Option<String>,
    pub path: Option<PathBuf>,
    pub hide_merges: bool,
    pub first_parent_only: bool,
}

impl CommitFilter {
    pub const fn new() -> Self {
        Self {
            refs: None,
            date_range: None,
            author: None,
            path: None,
            hide_merges: false,
            first_parent_only: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DateRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedRepoData {
    pub ref_snapshot: HashMap<String, Oid>,
    pub commit_summaries: Vec<CachedCommitSummary>,
    pub version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedCommitSummary {
    pub oid: Oid,
    pub summary: String,
    pub author: Author,
    pub author_time: DateTime<Utc>,
    pub parent_oids: Vec<Oid>,
    pub refs: Vec<Ref>,
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

    #[test]
    fn oid_from_hex_valid() {
        let hex = "0123456789abcdeffedcba987654321000000001";
        let oid = Oid::from_hex(hex).expect("valid hex");
        assert_eq!(oid.to_hex(), hex);
    }

    #[test]
    fn oid_from_hex_roundtrip() {
        let bytes: [u8; 20] = [
            0xde, 0xad, 0xbe, 0xef, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99,
            0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let original = Oid::from_bytes(bytes);
        let from_hex = Oid::from_hex(&original.to_hex()).expect("valid hex");
        assert_eq!(original, from_hex);
    }

    #[test]
    fn oid_from_hex_invalid_length() {
        let err = Oid::from_hex("deadbeef").unwrap_err();
        assert!(matches!(err, OidError::InvalidLength(8)));
    }

    #[test]
    fn oid_from_hex_invalid_char() {
        let err = Oid::from_hex("ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ").unwrap_err();
        assert!(matches!(err, OidError::InvalidChar(0)));
    }

    #[test]
    fn oid_from_hex_uppercase() {
        let hex = "DEADBEEF00112233445566778899AABBCCDDEEFF";
        let oid = Oid::from_hex(hex).expect("valid uppercase hex");
        assert_eq!(oid.to_hex(), "deadbeef00112233445566778899aabbccddeeff");
    }
}
