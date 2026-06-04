use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};

use crate::error::{DiffError, GitError};
use crate::models::*;
use crate::repository::Repository;

pub struct GixRepository {
    inner: gix::ThreadSafeRepository,
    path: PathBuf,
}

impl GixRepository {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = gix::discover(path).map_err(|e| {
            // gix doesn't expose a structured error kind for this; string match is fragile
            // but currently the only way to distinguish "not a git repo" from other errors
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
        let head_id = match repo.head_id() {
            Ok(id) => id,
            Err(_) => return Ok(Vec::new()),
        };
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

    fn diff_summary(&self, from: Option<Oid>, to: Oid) -> Result<DiffSummary, DiffError> {
        let repo = self.thread_local();
        let to_tree = tree_for_oid(&repo, to)?;
        let from_tree = from.map(|oid| tree_for_oid(&repo, oid)).transpose()?;

        let gix_changes = repo
            .diff_tree_to_tree(from_tree.as_ref(), Some(&to_tree), None)
            .map_err(|e| DiffError::Gix(e.to_string()))?;

        let mut files = Vec::new();
        let mut total_additions = 0usize;
        let mut total_deletions = 0usize;

        for change in &gix_changes {
            let (path, old_path, change_type, is_binary) = change_to_file_change_parts(change);

            let (additions, deletions) = if is_binary {
                (0, 0)
            } else {
                count_lines_for_change(&repo, change)
            };

            total_additions += additions;
            total_deletions += deletions;

            files.push(FileDiffSummary {
                path,
                old_path,
                change_type,
                additions,
                deletions,
                is_binary,
            });
        }

        Ok(DiffSummary {
            files,
            stats: DiffStats {
                files_changed: gix_changes.len(),
                additions: total_additions,
                deletions: total_deletions,
            },
        })
    }

    fn file_diff(
        &self,
        from: Option<Oid>,
        to: Oid,
        path: &std::path::Path,
    ) -> Result<FileDiff, DiffError> {
        let repo = self.thread_local();
        let to_tree = tree_for_oid(&repo, to)?;
        let from_tree = from.map(|oid| tree_for_oid(&repo, oid)).transpose()?;

        let gix_changes = repo
            .diff_tree_to_tree(from_tree.as_ref(), Some(&to_tree), None)
            .map_err(|e| DiffError::Gix(e.to_string()))?;

        let change = gix_changes
            .iter()
            .find(|c| std::path::PathBuf::from(c.location().to_string()) == path)
            .ok_or_else(|| DiffError::ObjectNotFound(path.display().to_string()))?;

        let (path, old_path, change_type, is_binary) = change_to_file_change_parts(change);

        if is_binary {
            return Ok(FileDiff {
                path,
                old_path,
                hunks: Vec::new(),
                is_binary: true,
                old_size: None,
                new_size: None,
                truncated_at: None,
            });
        }

        let hunks = compute_hunks_for_change(&repo, change)?;

        let file_is_binary = hunks.is_empty() && matches!(change_type, ChangeType::Modified);

        Ok(FileDiff {
            path,
            old_path,
            hunks,
            is_binary: file_is_binary,
            old_size: None,
            new_size: None,
            truncated_at: None,
        })
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
        other => unreachable!("unsupported hash algorithm: {:?}", other),
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
    let empty_sig = gix::actor::SignatureRef {
        name: "".into(),
        email: "".into(),
        time: "",
    };
    let author_sig = commit.author().unwrap_or(empty_sig.trim());
    let committer_sig = commit.committer().unwrap_or(empty_sig.trim());
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

fn tree_for_oid(repo: &gix::Repository, oid: Oid) -> Result<gix::Tree<'_>, DiffError> {
    let gix_oid = oid_to_gix_object_id(&oid);
    let obj = repo
        .find_object(gix_oid)
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    let commit = obj
        .try_into_commit()
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    commit.tree().map_err(|e| DiffError::Gix(e.to_string()))
}

fn change_to_file_change_parts(
    change: &gix::object::tree::diff::ChangeDetached,
) -> (
    std::path::PathBuf,
    Option<std::path::PathBuf>,
    ChangeType,
    bool,
) {
    match change {
        gix::object::tree::diff::ChangeDetached::Addition {
            entry_mode,
            location,
            ..
        } => (
            std::path::PathBuf::from(location.to_string()),
            None,
            ChangeType::Added,
            is_entry_mode_binary(*entry_mode),
        ),
        gix::object::tree::diff::ChangeDetached::Deletion {
            entry_mode,
            location,
            ..
        } => (
            std::path::PathBuf::from(location.to_string()),
            None,
            ChangeType::Deleted,
            is_entry_mode_binary(*entry_mode),
        ),
        gix::object::tree::diff::ChangeDetached::Modification {
            previous_entry_mode,
            entry_mode,
            location,
            ..
        } => (
            std::path::PathBuf::from(location.to_string()),
            None,
            ChangeType::Modified,
            is_entry_mode_binary(*previous_entry_mode) || is_entry_mode_binary(*entry_mode),
        ),
        gix::object::tree::diff::ChangeDetached::Rewrite {
            source_location,
            location,
            copy,
            ..
        } => (
            std::path::PathBuf::from(location.to_string()),
            Some(std::path::PathBuf::from(source_location.to_string())),
            if *copy {
                ChangeType::Copied
            } else {
                ChangeType::Renamed
            },
            false,
        ),
    }
}

fn is_entry_mode_binary(mode: gix_object::tree::EntryMode) -> bool {
    matches!(mode.kind(), gix_object::tree::EntryKind::Commit)
}

fn count_lines_for_change(
    repo: &gix::Repository,
    change: &gix::object::tree::diff::ChangeDetached,
) -> (usize, usize) {
    let location = change.location();

    match change {
        gix::object::tree::diff::ChangeDetached::Addition { id, entry_mode, .. } => {
            if is_entry_mode_binary(*entry_mode) {
                return (0, 0);
            }
            let line_count = count_blob_lines(repo, id);
            (line_count, 0)
        }
        gix::object::tree::diff::ChangeDetached::Deletion { id, entry_mode, .. } => {
            if is_entry_mode_binary(*entry_mode) {
                return (0, 0);
            }
            let line_count = count_blob_lines(repo, id);
            (0, line_count)
        }
        gix::object::tree::diff::ChangeDetached::Modification {
            previous_id,
            previous_entry_mode,
            id,
            entry_mode,
            ..
        } => {
            if is_entry_mode_binary(*previous_entry_mode) || is_entry_mode_binary(*entry_mode) {
                return (0, 0);
            }
            diff_line_counts(repo, location, previous_id, id)
        }
        gix::object::tree::diff::ChangeDetached::Rewrite {
            source_id,
            source_entry_mode,
            id,
            entry_mode,
            ..
        } => {
            if is_entry_mode_binary(*source_entry_mode) || is_entry_mode_binary(*entry_mode) {
                return (0, 0);
            }
            diff_line_counts(repo, location, source_id, id)
        }
    }
}

fn count_blob_lines(repo: &gix::Repository, id: &gix::hash::ObjectId) -> usize {
    let obj = match repo.find_object(*id) {
        Ok(o) => o,
        Err(_) => return 0,
    };
    let data = obj.data.as_slice();
    if data.iter().take(8192).any(|&b| b == 0) {
        return 0;
    }
    data.iter().filter(|&&b| b == b'\n').count()
        + if data.last() == Some(&b'\n') || data.is_empty() {
            0
        } else {
            1
        }
}

fn diff_line_counts(
    repo: &gix::Repository,
    location: &gix::bstr::BStr,
    source_id: &gix::hash::ObjectId,
    dest_id: &gix::hash::ObjectId,
) -> (usize, usize) {
    let mut cache = match repo.diff_resource_cache_for_tree_diff() {
        Ok(c) => c,
        Err(_) => return (0, 0),
    };

    if cache
        .set_resource(
            *source_id,
            gix_object::tree::EntryKind::Blob,
            location,
            gix_diff::blob::ResourceKind::OldOrSource,
            &repo.objects,
        )
        .is_err()
    {
        return (0, 0);
    }

    if cache
        .set_resource(
            *dest_id,
            gix_object::tree::EntryKind::Blob,
            location,
            gix_diff::blob::ResourceKind::NewOrDestination,
            &repo.objects,
        )
        .is_err()
    {
        return (0, 0);
    }

    let mut additions = 0usize;
    let mut deletions = 0usize;

    let mut diff_platform = gix::object::blob::diff::Platform {
        resource_cache: &mut cache,
    };

    let result = diff_platform.lines(|line_change| {
        match line_change {
            gix::object::blob::diff::lines::Change::Addition { lines } => {
                additions += lines.len();
            }
            gix::object::blob::diff::lines::Change::Deletion { lines } => {
                deletions += lines.len();
            }
            gix::object::blob::diff::lines::Change::Modification {
                lines_before,
                lines_after,
            } => {
                deletions += lines_before.len();
                additions += lines_after.len();
            }
        }
        Ok::<(), std::convert::Infallible>(())
    });

    if result.is_err() {
        return (0, 0);
    }

    (additions, deletions)
}

fn compute_hunks_for_change(
    repo: &gix::Repository,
    change: &gix::object::tree::diff::ChangeDetached,
) -> Result<Vec<Hunk>, DiffError> {
    let location = change.location();
    let (source_id, dest_id): (&gix::hash::ObjectId, &gix::hash::ObjectId) = match change {
        gix::object::tree::diff::ChangeDetached::Modification {
            previous_id, id, ..
        } => (previous_id, id),
        gix::object::tree::diff::ChangeDetached::Addition { id, .. } => {
            return compute_hunks_for_addition(repo, id);
        }
        gix::object::tree::diff::ChangeDetached::Deletion { id, .. } => {
            return compute_hunks_for_deletion(repo, id);
        }
        gix::object::tree::diff::ChangeDetached::Rewrite { source_id, id, .. } => (source_id, id),
    };

    let mut cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    cache
        .set_resource(
            *source_id,
            gix_object::tree::EntryKind::Blob,
            location,
            gix_diff::blob::ResourceKind::OldOrSource,
            &repo.objects,
        )
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    cache
        .set_resource(
            *dest_id,
            gix_object::tree::EntryKind::Blob,
            location,
            gix_diff::blob::ResourceKind::NewOrDestination,
            &repo.objects,
        )
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    run_blob_diff(&mut cache)
}

fn compute_hunks_for_addition(
    repo: &gix::Repository,
    id: &gix::hash::ObjectId,
) -> Result<Vec<Hunk>, DiffError> {
    let obj = repo
        .find_object(*id)
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    let data = obj.data.as_slice();
    if data.iter().take(8192).any(|&b| b == 0) {
        return Ok(Vec::new());
    }
    let lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    let mut diff_lines = Vec::new();
    for (line_num, line) in (1usize..).zip(lines.iter()) {
        if line.is_empty() && diff_lines.len() == lines.len() - 1 {
            break;
        }
        diff_lines.push(DiffLine::Addition {
            content: String::from_utf8_lossy(line).into_owned(),
            new_line: line_num,
        });
    }
    if diff_lines.is_empty() {
        return Ok(Vec::new());
    }
    Ok(vec![Hunk {
        old_start: 0,
        old_count: 0,
        new_start: 1,
        new_count: diff_lines.len(),
        lines: diff_lines,
    }])
}

fn compute_hunks_for_deletion(
    repo: &gix::Repository,
    id: &gix::hash::ObjectId,
) -> Result<Vec<Hunk>, DiffError> {
    let obj = repo
        .find_object(*id)
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    let data = obj.data.as_slice();
    if data.iter().take(8192).any(|&b| b == 0) {
        return Ok(Vec::new());
    }
    let lines: Vec<&[u8]> = data.split(|&b| b == b'\n').collect();
    let mut diff_lines = Vec::new();
    for (line_num, line) in (1usize..).zip(lines.iter()) {
        if line.is_empty() && diff_lines.len() == lines.len() - 1 {
            break;
        }
        diff_lines.push(DiffLine::Deletion {
            content: String::from_utf8_lossy(line).into_owned(),
            old_line: line_num,
        });
    }
    if diff_lines.is_empty() {
        return Ok(Vec::new());
    }
    Ok(vec![Hunk {
        old_start: 1,
        old_count: diff_lines.len(),
        new_start: 0,
        new_count: 0,
        lines: diff_lines,
    }])
}

fn run_blob_diff(cache: &mut gix_diff::blob::Platform) -> Result<Vec<Hunk>, DiffError> {
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut current_lines: Vec<DiffLine> = Vec::new();
    let mut old_line = 1usize;
    let mut new_line = 1usize;
    let mut hunk_old_start = 0usize;
    let mut hunk_new_start = 0usize;
    let mut hunk_old_count = 0usize;
    let mut hunk_new_count = 0usize;
    let mut has_content = false;

    let mut diff_platform = gix::object::blob::diff::Platform {
        resource_cache: cache,
    };

    let result = diff_platform.lines(|line_change| {
        let is_new_hunk = !current_lines.is_empty()
            && match line_change {
                gix::object::blob::diff::lines::Change::Addition { .. } => {
                    matches!(current_lines.last(), Some(DiffLine::Deletion { .. }))
                }
                gix::object::blob::diff::lines::Change::Deletion { .. } => {
                    matches!(current_lines.last(), Some(DiffLine::Addition { .. }))
                }
                gix::object::blob::diff::lines::Change::Modification { .. } => false,
            };

        if is_new_hunk {
            hunks.push(Hunk {
                old_start: hunk_old_start,
                old_count: hunk_old_count,
                new_start: hunk_new_start,
                new_count: hunk_new_count,
                lines: std::mem::take(&mut current_lines),
            });
            hunk_old_count = 0;
            hunk_new_count = 0;
            has_content = false;
        }

        if !has_content {
            hunk_old_start = old_line;
            hunk_new_start = new_line;
            has_content = true;
        }

        match line_change {
            gix::object::blob::diff::lines::Change::Addition { lines } => {
                for l in lines {
                    current_lines.push(DiffLine::Addition {
                        content: l.to_string(),
                        new_line,
                    });
                    new_line += 1;
                    hunk_new_count += 1;
                }
            }
            gix::object::blob::diff::lines::Change::Deletion { lines } => {
                for l in lines {
                    current_lines.push(DiffLine::Deletion {
                        content: l.to_string(),
                        old_line,
                    });
                    old_line += 1;
                    hunk_old_count += 1;
                }
            }
            gix::object::blob::diff::lines::Change::Modification {
                lines_before,
                lines_after,
            } => {
                for l in lines_before {
                    current_lines.push(DiffLine::Deletion {
                        content: l.to_string(),
                        old_line,
                    });
                    old_line += 1;
                    hunk_old_count += 1;
                }
                for l in lines_after {
                    current_lines.push(DiffLine::Addition {
                        content: l.to_string(),
                        new_line,
                    });
                    new_line += 1;
                    hunk_new_count += 1;
                }
            }
        }

        Ok::<(), std::convert::Infallible>(())
    });

    result.map_err(|e| DiffError::Gix(e.to_string()))?;

    if !current_lines.is_empty() {
        hunks.push(Hunk {
            old_start: hunk_old_start,
            old_count: hunk_old_count,
            new_start: hunk_new_start,
            new_count: hunk_new_count,
            lines: current_lines,
        });
    }

    Ok(hunks)
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

    #[test]
    fn diff_summary_root_commit_shows_all_added() {
        let temp = TempRepo::new();
        let oid = temp.commit_file("a.txt", "hello", "first commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let summary = repo.diff_summary(None, oid).expect("diff_summary");
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].path, std::path::PathBuf::from("a.txt"));
        assert_eq!(summary.files[0].change_type, ChangeType::Added);
        assert!(summary.files[0].additions > 0);
        assert_eq!(summary.files[0].deletions, 0);
        assert!(!summary.files[0].is_binary);
    }

    #[test]
    fn diff_summary_between_two_commits() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "hello", "first commit");
        let oid2 = temp.commit_file("b.txt", "world", "second commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let summary = repo.diff_summary(Some(oid1), oid2).expect("diff_summary");
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].path, std::path::PathBuf::from("b.txt"));
        assert_eq!(summary.files[0].change_type, ChangeType::Added);
        assert_eq!(summary.stats.files_changed, 1);
    }

    #[test]
    fn diff_summary_modification() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "hello", "first commit");
        let oid2 = temp.commit_file("a.txt", "hello world", "modify a");
        let repo = GixRepository::open(temp.path()).expect("open");
        let summary = repo.diff_summary(Some(oid1), oid2).expect("diff_summary");
        assert_eq!(summary.files.len(), 1);
        assert_eq!(summary.files[0].change_type, ChangeType::Modified);
        assert!(summary.files[0].additions > 0 || summary.files[0].deletions > 0);
    }

    #[test]
    fn file_diff_returns_hunks() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "line1\nline2\nline3", "first");
        let oid2 = temp.commit_file("a.txt", "line1\nmodified\nline3", "second");
        let repo = GixRepository::open(temp.path()).expect("open");
        let diff = repo
            .file_diff(Some(oid1), oid2, std::path::Path::new("a.txt"))
            .expect("file_diff");
        assert!(!diff.is_binary);
        assert!(!diff.hunks.is_empty());
        assert!(diff.hunks.iter().all(|h| !h.lines.is_empty()));
    }

    #[test]
    fn file_diff_not_found() {
        let temp = TempRepo::new();
        let oid = temp.commit_file("a.txt", "hello", "first");
        let repo = GixRepository::open(temp.path()).expect("open");
        let result = repo.file_diff(None, oid, std::path::Path::new("nonexistent.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn diff_summary_empty_when_identical() {
        let temp = TempRepo::new();
        let oid = temp.commit_file("a.txt", "hello", "first");
        let repo = GixRepository::open(temp.path()).expect("open");
        let summary = repo.diff_summary(Some(oid), oid).expect("diff_summary");
        assert!(summary.files.is_empty());
        assert_eq!(summary.stats.files_changed, 0);
    }
}
