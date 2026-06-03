use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};

use crate::error::GitError;
use crate::models::*;
use crate::repository::Repository;

pub struct GixRepository {
    inner: gix::ThreadSafeRepository,
    path: PathBuf,
}

impl GixRepository {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = gix::discover(path).map_err(|e| {
            if e.to_string().contains("not a git repository") {
                GitError::NotAGitRepository(path.display().to_string())
            } else {
                GitError::Gix(e.to_string())
            }
        })?;
        Ok(Self {
            inner: repo.into_sync(),
            path: path.to_path_buf(),
        })
    }

    fn thread_local(&self) -> gix::Repository {
        let mut repo = self.inner.to_thread_local();
        repo.object_cache_size(10 * 1024 * 1024);
        repo
    }
}

impl Repository for GixRepository {
    fn info(&self) -> Result<RepositoryInfo, GitError> {
        let repo = self.thread_local();
        let head_commit = repo.head_id().ok().map(|id| gix_id_to_oid(&id));
        let head_branch = repo
            .head_name()
            .ok()
            .flatten()
            .map(|name| name.shorten().to_string());
        let is_bare = repo.is_bare();
        Ok(RepositoryInfo {
            path: self.path.clone(),
            head_branch,
            head_commit,
            is_bare,
        })
    }

    fn commits(&self, max_count: Option<usize>) -> Result<Vec<CommitInfo>, GitError> {
        let repo = self.thread_local();
        let head_id = repo.head_id().map_err(|e| GitError::Gix(e.to_string()))?;
        let refs = build_ref_map(&repo)?;
        let walk = head_id
            .ancestors()
            .sorting(gix::revision::walk::Sorting::BreadthFirst)
            .all()
            .map_err(|e| GitError::Gix(e.to_string()))?;

        let mut result = Vec::new();
        for (count, info_result) in walk.enumerate() {
            if let Some(max) = max_count
                && count >= max
            {
                break;
            }
            let info = info_result.map_err(|e| GitError::Gix(e.to_string()))?;
            let oid = gix_object_id_to_oid(info.id);
            let commit = info.object().map_err(|e| GitError::Gix(e.to_string()))?;
            let commit_refs = refs.get(&oid).cloned().unwrap_or_default();
            result.push(commit_to_commit_info(&oid, &commit, commit_refs));
        }
        Ok(result)
    }

    fn commit(&self, oid: Oid) -> Result<CommitDetails, GitError> {
        let repo = self.thread_local();
        let gix_oid = oid_to_gix_object_id(&oid);
        let obj = repo
            .find_object(gix_oid)
            .map_err(|e| GitError::Gix(e.to_string()))?;
        let commit = obj
            .try_into_commit()
            .map_err(|e| GitError::InvalidObject(e.to_string()))?;
        let tree_id = commit.tree_id().map_err(|e| GitError::Gix(e.to_string()))?;
        let tree_oid = gix_id_to_oid(&tree_id);
        let refs = build_ref_map(&repo)?;
        let commit_refs = refs.get(&oid).cloned().unwrap_or_default();
        let info = commit_to_commit_info(&oid, &commit, commit_refs);
        let message = commit
            .message_raw()
            .map_err(|e| GitError::Gix(e.to_string()))?
            .to_string();
        let body = if message.lines().count() > 1 {
            Some(
                message
                    .lines()
                    .skip(1)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .trim_start_matches('\n')
                    .to_string(),
            )
        } else {
            None
        };

        Ok(CommitDetails {
            info,
            tree_oid,
            signature: None,
            changed_files: Vec::new(),
            body,
        })
    }

    fn refs(&self) -> Result<Vec<Ref>, GitError> {
        let repo = self.thread_local();
        let head_id = repo.head_id().ok();
        let mut result = Vec::new();
        let platform = repo
            .references()
            .map_err(|e| GitError::Gix(e.to_string()))?;
        let iter = platform.all().map_err(|e| GitError::Gix(e.to_string()))?;
        for reference in iter {
            let mut reference = reference.map_err(|e| GitError::Gix(e.to_string()))?;
            let name = reference.name();
            let category = name.category().map(|c| c.prefix().to_string());
            let name_str = name.shorten().to_string();
            let target_id = match reference.peel_to_id() {
                Ok(id) => id,
                _ => continue,
            };
            let oid = gix_id_to_oid(&target_id);
            let is_head = head_id
                .as_ref()
                .map(|hid| *hid == target_id)
                .unwrap_or(false);
            if let Some(r#ref) =
                categorize_ref_from_parts(category.as_deref(), name_str, oid, is_head)
            {
                result.push(r#ref);
            }
        }
        Ok(result)
    }

    fn stash_list(&self) -> Result<Vec<StashEntry>, GitError> {
        Ok(Vec::new())
    }

    fn reflog(&self, _ref_name: Option<&str>) -> Result<Vec<ReflogEntry>, GitError> {
        Ok(Vec::new())
    }

    fn file_tree(&self, _at_commit: Option<Oid>) -> Result<FileTreeNode, GitError> {
        Ok(FileTreeNode {
            name: String::new(),
            path: PathBuf::new(),
            node_type: FileNodeType::Directory,
            children: Vec::new(),
            size: None,
        })
    }

    fn is_bare(&self) -> bool {
        self.thread_local().is_bare()
    }
}

fn build_ref_map(repo: &gix::Repository) -> Result<HashMap<Oid, Vec<Ref>>, GitError> {
    let mut map: HashMap<Oid, Vec<Ref>> = HashMap::new();
    let head_id = repo.head_id().ok();
    let platform = repo
        .references()
        .map_err(|e| GitError::Gix(e.to_string()))?;
    let iter = platform.all().map_err(|e| GitError::Gix(e.to_string()))?;
    for reference in iter {
        let mut reference = reference.map_err(|e| GitError::Gix(e.to_string()))?;
        let name = reference.name();
        let category = name.category().map(|c| c.prefix().to_string());
        let name_str = name.shorten().to_string();
        let target_id = match reference.peel_to_id() {
            Ok(id) => id,
            _ => continue,
        };
        let oid = gix_id_to_oid(&target_id);
        let is_head = head_id
            .as_ref()
            .map(|hid| *hid == target_id)
            .unwrap_or(false);
        if let Some(r#ref) = categorize_ref_from_parts(category.as_deref(), name_str, oid, is_head)
        {
            map.entry(oid).or_default().push(r#ref);
        }
    }
    Ok(map)
}

fn categorize_ref_from_parts(
    category: Option<&str>,
    name_str: String,
    oid: Oid,
    is_head: bool,
) -> Option<Ref> {
    match category {
        Some("refs/heads/") => Some(Ref::Branch(BranchRef {
            name: name_str,
            is_head,
            is_remote: false,
            upstream: None,
            ahead: 0,
            behind: 0,
        })),
        Some("refs/remotes/") => {
            let parts: Vec<&str> = name_str.splitn(2, '/').collect();
            let (remote, branch_name) = match parts.as_slice() {
                [r, n] => (*r, *n),
                [n] => ("origin", *n),
                _ => return None,
            };
            Some(Ref::Remote(RemoteRef {
                name: branch_name.to_string(),
                remote: remote.to_string(),
            }))
        }
        Some("refs/tags/") => Some(Ref::Tag(TagRef {
            name: name_str,
            oid,
            annotation: None,
        })),
        _ => None,
    }
}

fn gix_id_to_oid(id: &gix::Id) -> Oid {
    gix_object_id_to_oid(id.detach())
}

fn gix_object_id_to_oid(oid: gix::ObjectId) -> Oid {
    match oid {
        gix::ObjectId::Sha1(bytes) => Oid::from_bytes(bytes),
        _ => Oid::from_bytes([0u8; 20]),
    }
}

pub(crate) fn oid_to_gix_object_id(oid: &Oid) -> gix::ObjectId {
    gix::ObjectId::from(*oid.as_bytes())
}

fn gix_signature_to_author(sig: &gix::actor::SignatureRef) -> Author {
    Author {
        name: sig.name.to_string(),
        email: sig.email.to_string(),
    }
}

fn gix_time_to_datetime(time: &gix::date::Time) -> chrono::DateTime<Utc> {
    chrono::Utc
        .timestamp_opt(time.seconds, 0)
        .single()
        .unwrap_or_default()
}

fn commit_to_commit_info(oid: &Oid, commit: &gix::Commit, refs: Vec<Ref>) -> CommitInfo {
    let default_sig = gix::actor::SignatureRef {
        name: "".into(),
        email: "".into(),
        time: "",
    };
    let author_sig = commit.author().unwrap_or(default_sig.trim());
    let default_sig2 = gix::actor::SignatureRef {
        name: "".into(),
        email: "".into(),
        time: "",
    };
    let committer_sig = commit.committer().unwrap_or(default_sig2.trim());
    let author = gix_signature_to_author(&author_sig);
    let committer = gix_signature_to_author(&committer_sig);
    let author_time = author_sig
        .time()
        .map(|t| gix_time_to_datetime(&t))
        .unwrap_or_default();
    let commit_time = committer_sig
        .time()
        .map(|t| gix_time_to_datetime(&t))
        .unwrap_or_default();
    let message = commit
        .message_raw()
        .map(|m| m.to_string())
        .unwrap_or_default();
    let summary = message.lines().next().unwrap_or("").to_string();
    let parent_oids: Vec<Oid> = commit.parent_ids().map(|id| gix_id_to_oid(&id)).collect();

    CommitInfo {
        oid: *oid,
        short_oid: oid.short_hex(),
        message,
        summary,
        author,
        committer,
        author_time,
        commit_time,
        parent_oids,
        refs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    struct TempRepo {
        dir: tempfile::TempDir,
    }

    impl TempRepo {
        fn new() -> Self {
            let dir = tempfile::TempDir::new().expect("temp dir");
            let path = dir.path();
            run_git(path, &["init"]);
            run_git(path, &["config", "user.name", "Test"]);
            run_git(path, &["config", "user.email", "test@test.com"]);
            Self { dir }
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        fn commit_file(&self, name: &str, content: &str, msg: &str) -> Oid {
            let file_path = self.dir.path().join(name);
            std::fs::write(&file_path, content).expect("write file");
            run_git(self.path(), &["add", name]);
            run_git(self.path(), &["commit", "-m", msg]);
            let output = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(self.path())
                .output()
                .expect("git rev-parse");
            let hex = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Oid::from_hex(&hex).expect("valid oid")
        }
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GIT_AUTHOR_DATE", "2025-01-01T00:00:00+0000")
            .env("GIT_COMMITTER_DATE", "2025-01-01T00:00:00+0000")
            .status()
            .expect("git command");
        assert!(status.success(), "git {:?} failed", args);
    }

    #[test]
    fn open_valid_repo() {
        let temp = TempRepo::new();
        let repo = GixRepository::open(temp.path());
        assert!(repo.is_ok(), "should open valid repo");
    }

    #[test]
    fn open_invalid_path() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let result = GixRepository::open(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn info_returns_correct_head() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let info = repo.info().expect("info");
        assert!(info.head_commit.is_some());
        assert!(!info.is_bare);
    }

    #[test]
    fn commits_returns_results() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        temp.commit_file("b.txt", "world", "second commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let commits = repo.commits(None).expect("commits");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "second commit");
        assert_eq!(commits[1].summary, "first commit");
        assert_eq!(commits[0].parent_oids.len(), 1);
        assert_eq!(commits[0].parent_oids[0], commits[1].oid);
    }

    #[test]
    fn commits_respects_max_count() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        temp.commit_file("b.txt", "world", "second commit");
        temp.commit_file("c.txt", "!", "third commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let commits = repo.commits(Some(2)).expect("commits");
        assert_eq!(commits.len(), 2);
    }

    #[test]
    fn commit_details_for_known_oid() {
        let temp = TempRepo::new();
        let oid = temp.commit_file("a.txt", "hello", "first commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let details = repo.commit(oid).expect("commit details");
        assert_eq!(details.info.oid, oid);
        assert_eq!(details.info.summary, "first commit");
        assert!(details.info.parent_oids.is_empty());
        assert!(details.tree_oid != Oid::from_bytes([0u8; 20]));
    }

    #[test]
    fn refs_includes_branch() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let refs = repo.refs().expect("refs");
        let has_branch = refs
            .iter()
            .any(|r| matches!(r, Ref::Branch(b) if b.is_head));
        assert!(has_branch, "should have at least one HEAD branch ref");
    }

    #[test]
    fn is_bare_false_for_normal_repo() {
        let temp = TempRepo::new();
        let repo = GixRepository::open(temp.path()).expect("open");
        assert!(!repo.is_bare());
    }
}
