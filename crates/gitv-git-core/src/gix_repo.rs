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

        let to_tree = tree_for_oid(&repo, oid).map_err(|e| GitError::Gix(e.to_string()))?;
        let parent_oid = info.parent_oids.first().copied();
        let from_tree = parent_oid
            .map(|p| tree_for_oid(&repo, p).map_err(|e| GitError::Gix(e.to_string())))
            .transpose()?;

        let gix_changes = repo
            .diff_tree_to_tree(from_tree.as_ref(), Some(&to_tree), None)
            .map_err(|e| GitError::Gix(e.to_string()))?;

        let mut changed_files = Vec::new();
        for change in &gix_changes {
            let Some((path, old_path, change_type, is_binary, is_submodule)) =
                change_to_file_change_parts(change)
            else {
                continue;
            };
            let (additions, deletions) = if is_binary || is_submodule {
                (0, 0)
            } else {
                count_lines_for_change(&repo, change)
            };
            changed_files.push(FileChange {
                path,
                old_path,
                change_type,
                additions,
                deletions,
                is_binary,
                is_submodule,
            });
        }

        Ok(CommitDetails {
            info,
            tree_oid,
            signature: None,
            changed_files,
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
        let repo = self.thread_local();
        let stash_ref = match repo
            .try_find_reference("stash")
            .map_err(|e| GitError::Gix(e.to_string()))?
        {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };
        let mut platform = stash_ref.log_iter();
        let rev_iter = match platform.rev().map_err(|e| GitError::Gix(e.to_string()))? {
            Some(iter) => iter,
            None => return Ok(Vec::new()),
        };

        let mut entries = Vec::new();
        for (index, entry_result) in rev_iter.enumerate() {
            let line = entry_result.map_err(|e| GitError::Gix(e.to_string()))?;
            let stash_oid = gix_object_id_to_oid(line.new_oid);
            let stash_obj = repo
                .find_object(line.new_oid)
                .map_err(|e| GitError::Gix(e.to_string()))?;
            let stash_commit = stash_obj
                .try_into_commit()
                .map_err(|e| GitError::Gix(e.to_string()))?;

            let parent_ids: Vec<gix::Id<'_>> = stash_commit.parent_ids().collect();
            let parent_oid = if let Some(first_parent) = parent_ids.first() {
                gix_id_to_oid(first_parent)
            } else {
                continue;
            };

            let author_sig = stash_commit.author().ok();
            let author = author_sig
                .as_ref()
                .map(|s| gix_signature_to_author(s))
                .unwrap_or(Author {
                    name: String::new(),
                    email: String::new(),
                });
            let time = author_sig
                .and_then(|s| s.time().ok())
                .map(|t| gix_time_to_datetime(&t))
                .unwrap_or_default();

            let message = stash_commit
                .message_raw()
                .map(|m| m.to_string())
                .unwrap_or_default();
            let summary = message.lines().next().unwrap_or("").to_string();

            let stash_tree = stash_commit
                .tree()
                .map_err(|e| GitError::Gix(e.to_string()))?;
            let file_summary = stash_file_summary_from_tree(&repo, &stash_tree, &parent_ids)?;

            entries.push(StashEntry {
                index,
                oid: stash_oid,
                parent_oid,
                message: summary,
                author,
                time,
                file_summary,
            });
        }

        Ok(entries)
    }

    fn stash_diff(&self, stash_index: usize) -> Result<FileDiff, DiffError> {
        let repo = self.thread_local();
        let (stash_oid, parent_tree) = resolve_stash(&repo, stash_index)?;
        let stash_obj = repo
            .find_object(stash_oid)
            .map_err(|e| DiffError::Gix(e.to_string()))?;
        let stash_commit = stash_obj
            .try_into_commit()
            .map_err(|e| DiffError::Gix(e.to_string()))?;
        let stash_tree = stash_commit
            .tree()
            .map_err(|e| DiffError::Gix(e.to_string()))?;

        let changes = repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&stash_tree), None)
            .map_err(|e| DiffError::Gix(e.to_string()))?;

        let mut all_hunks = Vec::new();
        let mut is_any_binary = false;
        let mut is_any_submodule = false;

        for change in &changes {
            let Some((_path, _old_path, _change_type, is_binary, is_submodule)) =
                change_to_file_change_parts(change)
            else {
                continue;
            };
            if is_binary || is_submodule {
                if is_binary {
                    is_any_binary = true;
                }
                if is_submodule {
                    is_any_submodule = true;
                }
                continue;
            }
            let (hunks, blob_binary) = compute_hunks_for_change(&repo, change)?;
            if blob_binary {
                is_any_binary = true;
            }
            if !hunks.is_empty() {
                all_hunks.extend(hunks);
            }
        }

        Ok(FileDiff {
            path: PathBuf::from(format!("stash@{{{stash_index}}}")),
            old_path: None,
            hunks: all_hunks,
            is_binary: is_any_binary,
            is_submodule: is_any_submodule,
            old_size: None,
            new_size: None,
            truncated_at: None,
        })
    }

    fn stash_split_diff(&self, stash_index: usize) -> Result<StashSplitDiff, DiffError> {
        let repo = self.thread_local();
        let (stash_oid, parent_tree) = resolve_stash(&repo, stash_index)?;
        let stash_obj = repo
            .find_object(stash_oid)
            .map_err(|e| DiffError::Gix(e.to_string()))?;
        let stash_commit = stash_obj
            .try_into_commit()
            .map_err(|e| DiffError::Gix(e.to_string()))?;
        let stash_tree = stash_commit
            .tree()
            .map_err(|e| DiffError::Gix(e.to_string()))?;

        let parent_ids: Vec<gix::Id<'_>> = stash_commit.parent_ids().collect();
        let index_tree = if let Some(index_id) = parent_ids.get(1) {
            let index_obj = repo
                .find_object(*index_id)
                .map_err(|e| DiffError::Gix(e.to_string()))?;
            let index_commit = index_obj
                .try_into_commit()
                .map_err(|e| DiffError::Gix(e.to_string()))?;
            Some(
                index_commit
                    .tree()
                    .map_err(|e| DiffError::Gix(e.to_string()))?,
            )
        } else {
            None
        };

        let staged = if let Some(ref idx_tree) = index_tree {
            compute_stash_half_diff(&repo, &parent_tree, idx_tree, stash_index, "staged")?
        } else {
            empty_stash_diff(stash_index, "staged")
        };

        let unstaged = compute_stash_half_diff(
            &repo,
            index_tree.as_ref().unwrap_or(&parent_tree),
            &stash_tree,
            stash_index,
            "unstaged",
        )?;

        Ok(StashSplitDiff { staged, unstaged })
    }

    fn reflog(&self, ref_name: Option<&str>) -> Result<Vec<ReflogEntry>, GitError> {
        let repo = self.thread_local();
        let name = ref_name.unwrap_or("HEAD");

        let mut entries = Vec::new();

        if name == "HEAD" {
            let head = repo.head().map_err(|e| GitError::Gix(e.to_string()))?;
            let mut log_platform = head.log_iter();
            let rev_iter = match log_platform
                .rev()
                .map_err(|e| GitError::Gix(e.to_string()))?
            {
                Some(iter) => iter,
                None => return Ok(entries),
            };
            for entry_result in rev_iter {
                let line = entry_result.map_err(|e| GitError::Gix(e.to_string()))?;
                entries.push(reflog_line_to_entry(line, "HEAD".to_string()));
            }
        } else {
            let reference = repo
                .try_find_reference(name)
                .map_err(|e| GitError::Gix(e.to_string()))?
                .ok_or_else(|| GitError::RefNotFound(name.to_string()))?;
            let mut log_platform = reference.log_iter();
            let rev_iter = match log_platform
                .rev()
                .map_err(|e| GitError::Gix(e.to_string()))?
            {
                Some(iter) => iter,
                None => return Ok(entries),
            };
            for entry_result in rev_iter {
                let line = entry_result.map_err(|e| GitError::Gix(e.to_string()))?;
                entries.push(reflog_line_to_entry(line, name.to_string()));
            }
        }

        Ok(entries)
    }

    fn blame(&self, path: &Path, at_commit: Option<Oid>) -> Result<Blame, GitError> {
        let repo = self.thread_local();
        let suspect = match at_commit {
            Some(oid) => oid_to_gix_object_id(&oid),
            None => repo
                .head_id()
                .map_err(|e| GitError::Gix(e.to_string()))?
                .detach(),
        };

        let file_path = gix::bstr::BString::from(path.to_string_lossy().into_owned());
        let file_bstr = gix::bstr::BStr::new(file_path.as_slice());
        let options = gix::repository::blame_file::Options::default();
        let outcome = repo
            .blame_file(file_bstr, suspect, options)
            .map_err(|e| GitError::Gix(e.to_string()))?;

        let mut lines = Vec::new();
        let mut line_num = 1usize;

        let mut commit_cache: HashMap<gix::ObjectId, (Author, chrono::DateTime<Utc>)> =
            HashMap::new();

        for (entry, entry_lines) in outcome.entries_with_lines() {
            let commit_oid = gix_object_id_to_oid(entry.commit_id);

            let (author, time) = match commit_cache.get(&entry.commit_id) {
                Some(cached) => cached.clone(),
                None => {
                    let commit_obj = repo
                        .find_object(entry.commit_id)
                        .map_err(|e| GitError::Gix(e.to_string()))?;
                    let commit = commit_obj
                        .try_into_commit()
                        .map_err(|e| GitError::Gix(e.to_string()))?;
                    let author_sig = commit.author().ok();
                    let author =
                        author_sig
                            .map(|s| gix_signature_to_author(&s))
                            .unwrap_or(Author {
                                name: String::new(),
                                email: String::new(),
                            });
                    let time = author_sig
                        .and_then(|s| s.time().ok())
                        .map(|t| gix_time_to_datetime(&t))
                        .unwrap_or_default();
                    commit_cache.insert(entry.commit_id, (author.clone(), time));
                    (author, time)
                }
            };

            for blame_line in entry_lines {
                let content = blame_line.to_string();
                lines.push(BlameLine {
                    line_number: line_num,
                    content,
                    commit_oid,
                    author: author.clone(),
                    time,
                });
                line_num += 1;
            }
        }

        Ok(Blame {
            file_path: path.to_path_buf(),
            lines,
        })
    }

    fn file_tree(&self, at_commit: Option<Oid>) -> Result<FileTreeNode, GitError> {
        let repo = self.thread_local();
        let tree = match at_commit {
            Some(oid) => {
                let gix_oid = oid_to_gix_object_id(&oid);
                let obj = repo
                    .find_object(gix_oid)
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let commit = obj
                    .try_into_commit()
                    .map_err(|e| GitError::InvalidObject(e.to_string()))?;
                commit.tree().map_err(|e| GitError::Gix(e.to_string()))?
            }
            None => {
                let head_id = repo.head_id().map_err(|e| GitError::Gix(e.to_string()))?;
                let obj = repo
                    .find_object(head_id)
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let commit = obj
                    .try_into_commit()
                    .map_err(|e| GitError::InvalidObject(e.to_string()))?;
                commit.tree().map_err(|e| GitError::Gix(e.to_string()))?
            }
        };
        build_file_tree(&repo, &tree, PathBuf::new())
    }

    fn is_bare(&self) -> bool {
        self.thread_local().is_bare()
    }

    fn blob_content(&self, oid: Oid, path: &Path) -> Result<String, GitError> {
        let repo = self.thread_local();
        let gix_oid = oid_to_gix_object_id(&oid);
        let obj = repo
            .find_object(gix_oid)
            .map_err(|e| GitError::Gix(e.to_string()))?;
        let commit = obj
            .try_into_commit()
            .map_err(|e| GitError::InvalidObject(e.to_string()))?;
        let mut tree = commit.tree().map_err(|e| GitError::Gix(e.to_string()))?;

        let parts: Vec<std::ffi::OsString> = path.iter().map(|p| p.to_os_string()).collect();
        for (i, part) in parts.iter().enumerate() {
            let lossy = part.to_string_lossy();
            let name = gix::bstr::BStr::new(lossy.as_bytes());
            let entry = tree
                .iter()
                .find_map(|e| {
                    let e = e.ok()?;
                    if e.filename() == name { Some(e) } else { None }
                })
                .ok_or_else(|| GitError::ObjectNotFound(path.display().to_string()))?;

            if i == parts.len() - 1 {
                let blob = repo
                    .find_object(entry.oid())
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let blob_obj = blob
                    .try_into_blob()
                    .map_err(|e| GitError::InvalidObject(e.to_string()))?;
                return String::from_utf8(blob_obj.data.to_vec())
                    .map_err(|e| GitError::Gix(format!("blob is not valid UTF-8: {e}")));
            } else {
                let obj = repo
                    .find_object(entry.oid())
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                tree = obj
                    .try_into_tree()
                    .map_err(|e| GitError::InvalidObject(e.to_string()))?;
            }
        }

        Err(GitError::ObjectNotFound(path.display().to_string()))
    }

    fn diff_summary(
        &self,
        from: Option<Oid>,
        to: Oid,
        whitespace: WhitespaceMode,
    ) -> Result<DiffSummary, DiffError> {
        // TODO: whitespace filtering on line counts requires re-counting per file
        let _ = whitespace;
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
            let Some((path, old_path, change_type, is_binary, is_submodule)) =
                change_to_file_change_parts(change)
            else {
                continue;
            };

            let (additions, deletions) = if is_binary || is_submodule {
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

    fn file_diff_limited(
        &self,
        from: Option<Oid>,
        to: Oid,
        path: &std::path::Path,
        mode: DiffMode,
        whitespace: WhitespaceMode,
        line_limit: Option<usize>,
    ) -> Result<FileDiff, DiffError> {
        let repo = self.thread_local();
        let to_tree = tree_for_oid(&repo, to)?;
        let from_tree = from.map(|oid| tree_for_oid(&repo, oid)).transpose()?;

        let gix_changes = repo
            .diff_tree_to_tree(from_tree.as_ref(), Some(&to_tree), None)
            .map_err(|e| DiffError::Gix(e.to_string()))?;

        let change = gix_changes
            .iter()
            .find(|c| c.location() == gix::bstr::BStr::new(path.to_string_lossy().as_bytes()))
            .ok_or_else(|| DiffError::ObjectNotFound(path.display().to_string()))?;

        let (path, old_path, _change_type, is_binary, is_submodule) =
            change_to_file_change_parts(change)
                .ok_or_else(|| DiffError::ObjectNotFound(path.display().to_string()))?;

        if is_submodule {
            let (old_sha, new_sha) = extract_submodule_shas(change);
            let msg = format!(
                "Submodule path {}: updated {}..{}",
                path.display(),
                old_sha,
                new_sha
            );
            return Ok(FileDiff {
                path,
                old_path,
                hunks: vec![Hunk {
                    old_start: 0,
                    old_count: 1,
                    new_start: 0,
                    new_count: 1,
                    lines: vec![DiffLine::Addition {
                        content: msg,
                        new_line: 1,
                    }],
                }],
                is_binary: false,
                is_submodule: true,
                old_size: None,
                new_size: None,
                truncated_at: None,
            });
        }

        if is_binary {
            return Ok(FileDiff {
                path,
                old_path,
                hunks: Vec::new(),
                is_binary: true,
                is_submodule: false,
                old_size: None,
                new_size: None,
                truncated_at: None,
            });
        }

        let (hunks, blob_is_binary) = compute_hunks_for_change(&repo, change)?;

        let hunks = apply_diff_options(hunks, &mode, &whitespace);

        let limit = line_limit.unwrap_or(usize::MAX);
        let mut total_lines = 0usize;
        let mut kept_hunks = Vec::new();
        let mut truncated_at: Option<usize> = None;

        for hunk in hunks {
            let hunk_lines = hunk.lines.len();
            if total_lines + hunk_lines > limit {
                truncated_at = Some(total_lines);
                break;
            }
            total_lines += hunk_lines;
            kept_hunks.push(hunk);
        }

        Ok(FileDiff {
            path,
            old_path,
            hunks: kept_hunks,
            is_binary: is_binary || blob_is_binary,
            is_submodule: false,
            old_size: None,
            new_size: None,
            truncated_at,
        })
    }

    fn file_history(
        &self,
        path: &std::path::Path,
        max_count: Option<usize>,
    ) -> Result<Vec<FileHistoryEntry>, GitError> {
        let repo = self.thread_local();
        let head_id = match repo.head_id() {
            Ok(id) => id,
            Err(_) => return Ok(Vec::new()),
        };

        let walk = head_id
            .ancestors()
            .sorting(gix::revision::walk::Sorting::ByCommitTime(
                gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
            ))
            .all()
            .map_err(|e| GitError::Gix(e.to_string()))?;

        let mut entries = Vec::new();
        let mut current_path = gix::bstr::BString::from(path.to_string_lossy().into_owned());

        for info_result in walk {
            if let Some(max) = max_count
                && entries.len() >= max
            {
                break;
            }
            let info = info_result.map_err(|e| GitError::Gix(e.to_string()))?;
            let commit_oid = gix_object_id_to_oid(info.id);
            let commit = info.object().map_err(|e| GitError::Gix(e.to_string()))?;

            let first_parent = commit.parent_ids().next();
            let parent_tree = if let Some(pid) = first_parent {
                let parent_obj = repo
                    .find_object(pid)
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let parent_commit = parent_obj
                    .try_into_commit()
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                Some(
                    parent_commit
                        .tree()
                        .map_err(|e| GitError::Gix(e.to_string()))?,
                )
            } else {
                None
            };

            let commit_tree = commit.tree().map_err(|e| GitError::Gix(e.to_string()))?;

            let parent_tree_ref = parent_tree.as_ref();
            let changes = repo
                .diff_tree_to_tree(
                    parent_tree_ref.map(|t| t as &gix::Tree<'_>),
                    Some(&commit_tree),
                    None,
                )
                .map_err(|e| GitError::Gix(e.to_string()))?;

            let bstr_path = gix::bstr::BStr::new(current_path.as_slice());
            let mut found = false;
            let mut rename_to: Option<gix::bstr::BString> = None;

            for change in &changes {
                let location = change.location();
                match change {
                    gix::object::tree::diff::ChangeDetached::Modification { .. } => {
                        if location == bstr_path {
                            found = true;
                            break;
                        }
                    }
                    gix::object::tree::diff::ChangeDetached::Addition { .. } => {
                        if location == bstr_path {
                            found = true;
                            break;
                        }
                    }
                    gix::object::tree::diff::ChangeDetached::Deletion { .. } => {
                        if location == bstr_path {
                            found = true;
                            break;
                        }
                    }
                    gix::object::tree::diff::ChangeDetached::Rewrite {
                        source_location,
                        location: new_location,
                        ..
                    } => {
                        if new_location == bstr_path {
                            found = true;
                            rename_to = Some(source_location.to_owned());
                            break;
                        }
                    }
                }
            }

            if found {
                let committer_sig = commit
                    .committer()
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let author = gix_signature_to_author(&committer_sig);
                let time = committer_sig
                    .time()
                    .map(|t| gix_time_to_datetime(&t))
                    .unwrap_or_default();
                let message = commit
                    .message_raw()
                    .map(|m| m.to_string())
                    .unwrap_or_default();
                let summary = message.lines().next().unwrap_or("").to_string();

                let entry_path =
                    std::path::PathBuf::from(String::from_utf8_lossy(&current_path).into_owned());
                let old_path = rename_to.as_ref().map(|p| {
                    let old = std::path::PathBuf::from(p.to_string());
                    current_path = p.clone();
                    old
                });

                entries.push(FileHistoryEntry {
                    commit_oid,
                    path: entry_path,
                    old_path,
                    summary,
                    author,
                    time,
                });
            }
        }

        Ok(entries)
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
) -> Option<(
    std::path::PathBuf,
    Option<std::path::PathBuf>,
    ChangeType,
    bool,
    bool,
)> {
    let is_tree =
        |mode: gix_object::tree::EntryMode| mode.kind() == gix_object::tree::EntryKind::Tree;
    let is_submodule_entry = |mode: gix_object::tree::EntryMode| {
        matches!(mode.kind(), gix_object::tree::EntryKind::Commit)
    };

    match change {
        gix::object::tree::diff::ChangeDetached::Addition {
            entry_mode,
            location,
            ..
        } => {
            if is_tree(*entry_mode) {
                return None;
            }
            let is_sub = is_submodule_entry(*entry_mode);
            Some((
                std::path::PathBuf::from(location.to_string()),
                None,
                if is_sub {
                    ChangeType::SubmoduleUpdated
                } else {
                    ChangeType::Added
                },
                false,
                is_sub,
            ))
        }
        gix::object::tree::diff::ChangeDetached::Deletion {
            entry_mode,
            location,
            ..
        } => {
            if is_tree(*entry_mode) {
                return None;
            }
            let is_sub = is_submodule_entry(*entry_mode);
            Some((
                std::path::PathBuf::from(location.to_string()),
                None,
                if is_sub {
                    ChangeType::SubmoduleUpdated
                } else {
                    ChangeType::Deleted
                },
                false,
                is_sub,
            ))
        }
        gix::object::tree::diff::ChangeDetached::Modification {
            previous_entry_mode,
            entry_mode,
            location,
            ..
        } => {
            if is_tree(*previous_entry_mode) || is_tree(*entry_mode) {
                return None;
            }
            let is_sub =
                is_submodule_entry(*previous_entry_mode) || is_submodule_entry(*entry_mode);
            Some((
                std::path::PathBuf::from(location.to_string()),
                None,
                if is_sub {
                    ChangeType::SubmoduleUpdated
                } else {
                    ChangeType::Modified
                },
                false,
                is_sub,
            ))
        }
        gix::object::tree::diff::ChangeDetached::Rewrite {
            source_location,
            location,
            copy,
            source_entry_mode,
            entry_mode,
            ..
        } => {
            if is_tree(*source_entry_mode) || is_tree(*entry_mode) {
                return None;
            }
            let is_sub = is_submodule_entry(*source_entry_mode) || is_submodule_entry(*entry_mode);
            Some((
                std::path::PathBuf::from(location.to_string()),
                Some(std::path::PathBuf::from(source_location.to_string())),
                if *copy {
                    ChangeType::Copied
                } else {
                    ChangeType::Renamed
                },
                false,
                is_sub,
            ))
        }
    }
}

fn is_entry_mode_submodule(mode: gix_object::tree::EntryMode) -> bool {
    matches!(mode.kind(), gix_object::tree::EntryKind::Commit)
}

fn extract_submodule_shas(change: &gix::object::tree::diff::ChangeDetached) -> (String, String) {
    match change {
        gix::object::tree::diff::ChangeDetached::Addition { id, .. } => {
            ("0000000".to_string(), id.to_hex_with_len(7).to_string())
        }
        gix::object::tree::diff::ChangeDetached::Deletion { id, .. } => {
            (id.to_hex_with_len(7).to_string(), "0000000".to_string())
        }
        gix::object::tree::diff::ChangeDetached::Modification {
            previous_id, id, ..
        } => (
            previous_id.to_hex_with_len(7).to_string(),
            id.to_hex_with_len(7).to_string(),
        ),
        gix::object::tree::diff::ChangeDetached::Rewrite { source_id, id, .. } => (
            source_id.to_hex_with_len(7).to_string(),
            id.to_hex_with_len(7).to_string(),
        ),
    }
}

fn count_lines_for_change(
    repo: &gix::Repository,
    change: &gix::object::tree::diff::ChangeDetached,
) -> (usize, usize) {
    let location = change.location();

    match change {
        gix::object::tree::diff::ChangeDetached::Addition { id, entry_mode, .. } => {
            if is_entry_mode_submodule(*entry_mode) {
                return (0, 0);
            }
            let line_count = count_blob_lines(repo, id);
            (line_count, 0)
        }
        gix::object::tree::diff::ChangeDetached::Deletion { id, entry_mode, .. } => {
            if is_entry_mode_submodule(*entry_mode) {
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
            if is_entry_mode_submodule(*previous_entry_mode) || is_entry_mode_submodule(*entry_mode)
            {
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
            if is_entry_mode_submodule(*source_entry_mode) || is_entry_mode_submodule(*entry_mode) {
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
) -> Result<(Vec<Hunk>, bool), DiffError> {
    let location = change.location();
    let (source_id, dest_id): (&gix::hash::ObjectId, &gix::hash::ObjectId) = match change {
        gix::object::tree::diff::ChangeDetached::Modification {
            previous_id, id, ..
        } => (previous_id, id),
        gix::object::tree::diff::ChangeDetached::Addition { id, .. } => {
            return compute_hunks_for_addition(repo, id).map(|h| (h, false));
        }
        gix::object::tree::diff::ChangeDetached::Deletion { id, .. } => {
            return compute_hunks_for_deletion(repo, id).map(|h| (h, false));
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

    let result = run_blob_diff(&mut cache)?;
    Ok((result.hunks, result.is_binary))
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

struct BlobDiffResult {
    hunks: Vec<Hunk>,
    is_binary: bool,
}

fn run_blob_diff(cache: &mut gix_diff::blob::Platform) -> Result<BlobDiffResult, DiffError> {
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

    let outcome = result.map_err(|e| DiffError::Gix(e.to_string()))?;

    let is_binary = matches!(
        outcome.operation,
        gix_diff::blob::platform::prepare_diff::Operation::SourceOrDestinationIsBinary
    );

    if !current_lines.is_empty() {
        hunks.push(Hunk {
            old_start: hunk_old_start,
            old_count: hunk_old_count,
            new_start: hunk_new_start,
            new_count: hunk_new_count,
            lines: current_lines,
        });
    }

    Ok(BlobDiffResult { hunks, is_binary })
}

fn apply_diff_options(hunks: Vec<Hunk>, mode: &DiffMode, whitespace: &WhitespaceMode) -> Vec<Hunk> {
    let hunks = apply_whitespace_filter(hunks, whitespace);
    match mode {
        DiffMode::Normal => hunks,
        DiffMode::WordDiff => hunks_to_word_diff(hunks),
        DiffMode::StatOnly => Vec::new(),
    }
}

fn apply_whitespace_filter(hunks: Vec<Hunk>, mode: &WhitespaceMode) -> Vec<Hunk> {
    match mode {
        WhitespaceMode::None => hunks,
        WhitespaceMode::IgnoreSpaceChange => filter_whitespace_hunks(hunks, |old, new| {
            collapse_whitespace(old) == collapse_whitespace(new)
        }),
        WhitespaceMode::IgnoreAllSpace => filter_whitespace_hunks(hunks, |old, new| {
            remove_all_whitespace(old) == remove_all_whitespace(new)
        }),
        WhitespaceMode::IgnoreBlankLines => filter_blank_line_hunks(hunks),
    }
}

fn filter_whitespace_hunks<F>(hunks: Vec<Hunk>, is_same: F) -> Vec<Hunk>
where
    F: Fn(&str, &str) -> bool,
{
    fn flush_pending(
        kept: &mut Vec<DiffLine>,
        additions: &mut Vec<DiffLine>,
        deletions: &mut Vec<DiffLine>,
        is_same: &dyn Fn(&str, &str) -> bool,
    ) {
        if additions.is_empty() && deletions.is_empty() {
            return;
        }
        if !additions.is_empty() && !deletions.is_empty() {
            let del_content: String = deletions
                .iter()
                .filter_map(|l| match l {
                    DiffLine::Deletion { content, .. } => Some(content.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            let add_content: String = additions
                .iter()
                .filter_map(|l| match l {
                    DiffLine::Addition { content, .. } => Some(content.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            if is_same(&del_content, &add_content) {
                additions.clear();
                deletions.clear();
                return;
            }
        }
        kept.append(deletions);
        kept.append(additions);
    }

    let mut result = Vec::new();
    for hunk in hunks {
        let mut kept = Vec::new();
        let mut additions: Vec<DiffLine> = Vec::new();
        let mut deletions: Vec<DiffLine> = Vec::new();

        for line in hunk.lines {
            match &line {
                DiffLine::Addition { .. } => {
                    additions.push(line);
                }
                DiffLine::Deletion { .. } => {
                    deletions.push(line);
                }
                DiffLine::Context { .. } | DiffLine::WordDiff { .. } => {
                    flush_pending(&mut kept, &mut additions, &mut deletions, &is_same);
                    kept.push(line);
                }
            }
        }
        flush_pending(&mut kept, &mut additions, &mut deletions, &is_same);

        if !kept.is_empty() {
            result.push(rebuild_hunk(hunk.old_start, hunk.new_start, kept));
        }
    }
    result
}

fn rebuild_hunk(old_start: usize, new_start: usize, lines: Vec<DiffLine>) -> Hunk {
    let old_count = lines
        .iter()
        .filter(|l| matches!(l, DiffLine::Deletion { .. } | DiffLine::Context { .. }))
        .count();
    let new_count = lines
        .iter()
        .filter(|l| matches!(l, DiffLine::Addition { .. } | DiffLine::Context { .. }))
        .count();
    Hunk {
        old_start,
        old_count,
        new_start,
        new_count,
        lines,
    }
}

fn filter_blank_line_hunks(hunks: Vec<Hunk>) -> Vec<Hunk> {
    let mut result = Vec::new();
    for hunk in hunks {
        let kept: Vec<DiffLine> = hunk
            .lines
            .into_iter()
            .filter(|line| match line {
                DiffLine::Addition { content, .. } => !content.trim().is_empty(),
                DiffLine::Deletion { content, .. } => !content.trim().is_empty(),
                _ => true,
            })
            .collect();
        if !kept.is_empty() {
            result.push(rebuild_hunk(hunk.old_start, hunk.new_start, kept));
        }
    }
    result
}

fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_whitespace = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !in_whitespace {
                result.push(' ');
                in_whitespace = true;
            }
        } else {
            result.push(c);
            in_whitespace = false;
        }
    }
    result
}

fn remove_all_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn hunks_to_word_diff(hunks: Vec<Hunk>) -> Vec<Hunk> {
    hunks
        .into_iter()
        .map(|hunk| {
            let mut word_lines: Vec<DiffLine> = Vec::new();
            let mut i = 0;
            let lines = hunk.lines;
            while i < lines.len() {
                match &lines[i] {
                    DiffLine::Deletion { content, old_line } => {
                        let old_line = *old_line;
                        let old_content = content.clone();
                        if i + 1 < lines.len()
                            && let DiffLine::Addition {
                                content: new_content,
                                new_line,
                            } = &lines[i + 1]
                        {
                            let segments = compute_word_segments(&old_content, new_content);
                            word_lines.push(DiffLine::WordDiff {
                                content: new_content.clone(),
                                old_line,
                                new_line: *new_line,
                                segments,
                            });
                            i += 2;
                            continue;
                        }
                        let segments = vec![WordDiffSegment {
                            text: old_content,
                            kind: WordDiffKind::Removed,
                        }];
                        word_lines.push(DiffLine::WordDiff {
                            content: String::new(),
                            old_line,
                            new_line: 0,
                            segments,
                        });
                        i += 1;
                    }
                    DiffLine::Addition { content, new_line } => {
                        let segments = vec![WordDiffSegment {
                            text: content.clone(),
                            kind: WordDiffKind::Added,
                        }];
                        word_lines.push(DiffLine::WordDiff {
                            content: content.clone(),
                            old_line: 0,
                            new_line: *new_line,
                            segments,
                        });
                        i += 1;
                    }
                    DiffLine::Context {
                        content,
                        old_line,
                        new_line,
                    } => {
                        let segments = vec![WordDiffSegment {
                            text: content.clone(),
                            kind: WordDiffKind::Unchanged,
                        }];
                        word_lines.push(DiffLine::WordDiff {
                            content: content.clone(),
                            old_line: *old_line,
                            new_line: *new_line,
                            segments,
                        });
                        i += 1;
                    }
                    DiffLine::WordDiff { .. } => {
                        word_lines.push(lines[i].clone());
                        i += 1;
                    }
                }
            }
            Hunk {
                old_start: hunk.old_start,
                old_count: hunk.old_count,
                new_start: hunk.new_start,
                new_count: hunk.new_count,
                lines: word_lines,
            }
        })
        .collect()
}

fn compute_word_segments(old: &str, new: &str) -> Vec<WordDiffSegment> {
    let old_words = tokenize_words(old);
    let new_words = tokenize_words(new);

    let mut segments = Vec::new();
    let mut old_idx = 0usize;
    let mut new_idx = 0usize;

    while old_idx < old_words.len() || new_idx < new_words.len() {
        match (old_words.get(old_idx), new_words.get(new_idx)) {
            (Some(ow), Some(nw)) if ow == nw => {
                segments.push(WordDiffSegment {
                    text: ow.clone(),
                    kind: WordDiffKind::Unchanged,
                });
                old_idx += 1;
                new_idx += 1;
            }
            (Some(_), None) => {
                segments.push(WordDiffSegment {
                    text: old_words[old_idx].clone(),
                    kind: WordDiffKind::Removed,
                });
                old_idx += 1;
            }
            (None, Some(_)) => {
                segments.push(WordDiffSegment {
                    text: new_words[new_idx].clone(),
                    kind: WordDiffKind::Added,
                });
                new_idx += 1;
            }
            (None, None) => break,
            (Some(_), Some(_)) => {
                let ahead_old =
                    find_in_range(&new_words, new_idx + 1..new_idx + 4, &old_words[old_idx]);
                let ahead_new =
                    find_in_range(&old_words, old_idx + 1..old_idx + 4, &new_words[new_idx]);

                if let Some(ao) = ahead_old {
                    for word in new_words.iter().take(ao).skip(new_idx) {
                        segments.push(WordDiffSegment {
                            text: word.clone(),
                            kind: WordDiffKind::Added,
                        });
                    }
                    new_idx = ao;
                } else if let Some(an) = ahead_new {
                    for word in old_words.iter().take(an).skip(old_idx) {
                        segments.push(WordDiffSegment {
                            text: word.clone(),
                            kind: WordDiffKind::Removed,
                        });
                    }
                    old_idx = an;
                } else {
                    segments.push(WordDiffSegment {
                        text: old_words[old_idx].clone(),
                        kind: WordDiffKind::Removed,
                    });
                    segments.push(WordDiffSegment {
                        text: new_words[new_idx].clone(),
                        kind: WordDiffKind::Added,
                    });
                    old_idx += 1;
                    new_idx += 1;
                }
            }
        }
    }

    merge_adjacent_segments(segments)
}

fn find_in_range(
    words: &[String],
    mut range: std::ops::Range<usize>,
    target: &str,
) -> Option<usize> {
    range.find(|&i| i < words.len() && words[i] == target)
}

fn tokenize_words(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_word = false;

    for ch in text.chars() {
        if ch.is_whitespace() {
            if in_word {
                tokens.push(std::mem::take(&mut current));
                in_word = false;
            }
            current.push(ch);
        } else if ch.is_alphanumeric() || ch == '_' {
            if !in_word && !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            current.push(ch);
            in_word = true;
        } else {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            tokens.push(ch.to_string());
            in_word = false;
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn merge_adjacent_segments(segments: Vec<WordDiffSegment>) -> Vec<WordDiffSegment> {
    let mut merged: Vec<WordDiffSegment> = Vec::new();
    for seg in segments {
        if let Some(last) = merged.last_mut()
            && last.kind == seg.kind
        {
            last.text.push_str(&seg.text);
            continue;
        }
        merged.push(seg);
    }
    merged
}

fn reflog_line_to_entry(line: gix::refs::log::Line, ref_name: String) -> ReflogEntry {
    let oid = gix_object_id_to_oid(line.new_oid);
    let null_oid = gix::hash::ObjectId::null(gix::hash::Kind::Sha1);
    let old_oid = if line.previous_oid == null_oid {
        None
    } else {
        Some(gix_object_id_to_oid(line.previous_oid))
    };
    let author = Author {
        name: line.signature.name.to_string(),
        email: line.signature.email.to_string(),
    };
    let time = gix_time_to_datetime(&line.signature.time);
    ReflogEntry {
        oid,
        old_oid,
        ref_name,
        message: line.message.to_string(),
        author,
        time,
    }
}

fn resolve_stash(
    repo: &gix::Repository,
    stash_index: usize,
) -> Result<(gix::ObjectId, gix::Tree<'_>), DiffError> {
    let stash_ref = repo
        .try_find_reference("stash")
        .map_err(|e| DiffError::Gix(e.to_string()))?
        .ok_or_else(|| DiffError::ObjectNotFound("refs/stash not found".to_string()))?;
    let mut platform = stash_ref.log_iter();
    let mut rev_iter = platform
        .rev()
        .map_err(|e| DiffError::Gix(e.to_string()))?
        .ok_or_else(|| DiffError::ObjectNotFound("stash reflog empty".to_string()))?;

    let line = rev_iter
        .nth(stash_index)
        .ok_or_else(|| DiffError::ObjectNotFound(format!("stash@{{{stash_index}}} not found")))?
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    let stash_obj = repo
        .find_object(line.new_oid)
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    let stash_commit = stash_obj
        .try_into_commit()
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    let parent_ids: Vec<gix::Id<'_>> = stash_commit.parent_ids().collect();
    let first_parent_id = parent_ids
        .first()
        .ok_or_else(|| DiffError::ObjectNotFound("stash has no parent".to_string()))?;
    let parent_obj = repo
        .find_object(*first_parent_id)
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    let parent_commit = parent_obj
        .try_into_commit()
        .map_err(|e| DiffError::Gix(e.to_string()))?;
    let parent_tree = parent_commit
        .tree()
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    Ok((line.new_oid, parent_tree))
}

fn stash_file_summary_from_tree(
    repo: &gix::Repository,
    stash_tree: &gix::Tree<'_>,
    parent_ids: &[gix::Id<'_>],
) -> Result<Vec<StashFileSummary>, GitError> {
    let first_parent_id = match parent_ids.first() {
        Some(id) => *id,
        None => return Ok(Vec::new()),
    };
    let parent_obj = repo
        .find_object(first_parent_id)
        .map_err(|e| GitError::Gix(e.to_string()))?;
    let parent_commit = parent_obj
        .try_into_commit()
        .map_err(|e| GitError::Gix(e.to_string()))?;
    let parent_tree = parent_commit
        .tree()
        .map_err(|e| GitError::Gix(e.to_string()))?;

    let changes = repo
        .diff_tree_to_tree(Some(&parent_tree), Some(stash_tree), None)
        .map_err(|e| GitError::Gix(e.to_string()))?;

    let mut summary = Vec::new();
    for change in &changes {
        let Some((path, _, change_type, _, _)) = change_to_file_change_parts(change) else {
            continue;
        };
        let stash_change_type = match change_type {
            ChangeType::Added => StashChangeType::Added,
            ChangeType::Deleted => StashChangeType::Deleted,
            _ => StashChangeType::Modified,
        };
        summary.push(StashFileSummary {
            path,
            change_type: stash_change_type,
        });
    }
    Ok(summary)
}

fn empty_stash_diff(stash_index: usize, label: &str) -> FileDiff {
    FileDiff {
        path: PathBuf::from(format!("stash@{{{stash_index}}} ({label})")),
        old_path: None,
        hunks: Vec::new(),
        is_binary: false,
        is_submodule: false,
        old_size: None,
        new_size: None,
        truncated_at: None,
    }
}

fn compute_stash_half_diff(
    repo: &gix::Repository,
    from_tree: &gix::Tree<'_>,
    to_tree: &gix::Tree<'_>,
    stash_index: usize,
    label: &str,
) -> Result<FileDiff, DiffError> {
    let changes = repo
        .diff_tree_to_tree(Some(from_tree), Some(to_tree), None)
        .map_err(|e| DiffError::Gix(e.to_string()))?;

    let mut all_hunks = Vec::new();
    let mut is_any_binary = false;
    let mut is_any_submodule = false;

    for change in &changes {
        let Some((path, old_path, _change_type, is_binary, is_submodule)) =
            change_to_file_change_parts(change)
        else {
            continue;
        };
        if is_binary || is_submodule {
            if is_binary {
                is_any_binary = true;
            }
            if is_submodule {
                is_any_submodule = true;
            }
            continue;
        }
        let (hunks, blob_binary) = compute_hunks_for_change(repo, change)?;
        if blob_binary {
            is_any_binary = true;
        }
        if !hunks.is_empty() {
            all_hunks.extend(hunks);
        }
        let _ = (path, old_path);
    }

    Ok(FileDiff {
        path: PathBuf::from(format!("stash@{{{stash_index}}} ({label})")),
        old_path: None,
        hunks: all_hunks,
        is_binary: is_any_binary,
        is_submodule: is_any_submodule,
        old_size: None,
        new_size: None,
        truncated_at: None,
    })
}

fn build_file_tree(
    repo: &gix::Repository,
    tree: &gix::Tree<'_>,
    prefix: PathBuf,
) -> Result<FileTreeNode, GitError> {
    let mut children = Vec::new();

    for entry in tree.iter() {
        let entry = entry.map_err(|e| GitError::Gix(e.to_string()))?;
        let name = String::from_utf8_lossy(entry.filename()).into_owned();
        let path = if prefix.as_os_str().is_empty() {
            PathBuf::from(&name)
        } else {
            prefix.join(&name)
        };

        match entry.mode().kind() {
            gix_object::tree::EntryKind::Tree => {
                let obj = repo
                    .find_object(entry.oid())
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let sub_tree = obj
                    .try_into_tree()
                    .map_err(|e| GitError::Gix(e.to_string()))?;
                let child = build_file_tree(repo, &sub_tree, path.clone())?;
                children.push(child);
            }
            gix_object::tree::EntryKind::Blob | gix_object::tree::EntryKind::BlobExecutable => {
                children.push(FileTreeNode {
                    name,
                    path,
                    node_type: FileNodeType::File,
                    children: Vec::new(),
                    size: None,
                });
            }
            gix_object::tree::EntryKind::Link => {
                children.push(FileTreeNode {
                    name,
                    path,
                    node_type: FileNodeType::Symlink,
                    children: Vec::new(),
                    size: None,
                });
            }
            gix_object::tree::EntryKind::Commit => {
                children.push(FileTreeNode {
                    name,
                    path,
                    node_type: FileNodeType::Submodule,
                    children: Vec::new(),
                    size: None,
                });
            }
        }
    }

    children.sort_by(|a, b| {
        let a_is_dir = matches!(a.node_type, FileNodeType::Directory);
        let b_is_dir = matches!(b.node_type, FileNodeType::Directory);
        b_is_dir
            .cmp(&a_is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    let dir_name = if prefix.as_os_str().is_empty() {
        String::from("/")
    } else {
        prefix
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };

    Ok(FileTreeNode {
        name: dir_name,
        path: prefix,
        node_type: FileNodeType::Directory,
        children,
        size: None,
    })
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
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).expect("create dir");
            }
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
        let summary = repo
            .diff_summary(None, oid, WhitespaceMode::None)
            .expect("diff_summary");
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
        let summary = repo
            .diff_summary(Some(oid1), oid2, WhitespaceMode::None)
            .expect("diff_summary");
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
        let summary = repo
            .diff_summary(Some(oid1), oid2, WhitespaceMode::None)
            .expect("diff_summary");
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
            .file_diff(
                Some(oid1),
                oid2,
                std::path::Path::new("a.txt"),
                DiffMode::Normal,
                WhitespaceMode::None,
            )
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
        let result = repo.file_diff(
            None,
            oid,
            std::path::Path::new("nonexistent.txt"),
            DiffMode::Normal,
            WhitespaceMode::None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn diff_summary_empty_when_identical() {
        let temp = TempRepo::new();
        let oid = temp.commit_file("a.txt", "hello", "first");
        let repo = GixRepository::open(temp.path()).expect("open");
        let summary = repo
            .diff_summary(Some(oid), oid, WhitespaceMode::None)
            .expect("diff_summary");
        assert!(summary.files.is_empty());
        assert_eq!(summary.stats.files_changed, 0);
    }

    #[test]
    fn file_tree_returns_children() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first");
        temp.commit_file("dir/b.txt", "world", "add nested");
        let repo = GixRepository::open(temp.path()).expect("open");
        let tree = repo.file_tree(None).expect("file_tree");
        assert!(matches!(tree.node_type, FileNodeType::Directory));
        assert!(tree.children.len() >= 2);
        let has_dir = tree
            .children
            .iter()
            .any(|c| matches!(c.node_type, FileNodeType::Directory) && c.name == "dir");
        let has_file = tree
            .children
            .iter()
            .any(|c| matches!(c.node_type, FileNodeType::File) && c.name == "a.txt");
        assert!(has_dir, "should have dir directory");
        assert!(has_file, "should have a.txt file");
    }

    #[test]
    fn file_tree_at_commit_shows_state() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "hello", "first");
        let _oid2 = temp.commit_file("b.txt", "world", "second");
        let repo = GixRepository::open(temp.path()).expect("open");
        let tree = repo.file_tree(Some(oid1)).expect("file_tree");
        let has_a = tree
            .children
            .iter()
            .any(|c| matches!(c.node_type, FileNodeType::File) && c.name == "a.txt");
        let has_b = tree
            .children
            .iter()
            .any(|c| matches!(c.node_type, FileNodeType::File) && c.name == "b.txt");
        assert!(has_a, "first commit should have a.txt");
        assert!(!has_b, "first commit should not have b.txt");
    }

    #[test]
    fn stat_only_mode_returns_empty_hunks() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "line1\nline2\nline3", "first");
        let oid2 = temp.commit_file("a.txt", "line1\nmodified\nline3", "second");
        let repo = GixRepository::open(temp.path()).expect("open");
        let diff = repo
            .file_diff(
                Some(oid1),
                oid2,
                std::path::Path::new("a.txt"),
                DiffMode::StatOnly,
                WhitespaceMode::None,
            )
            .expect("file_diff");
        assert!(diff.hunks.is_empty());
    }

    #[test]
    fn word_diff_mode_returns_word_diff_lines() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "hello world", "first");
        let oid2 = temp.commit_file("a.txt", "hello earth", "second");
        let repo = GixRepository::open(temp.path()).expect("open");
        let diff = repo
            .file_diff(
                Some(oid1),
                oid2,
                std::path::Path::new("a.txt"),
                DiffMode::WordDiff,
                WhitespaceMode::None,
            )
            .expect("file_diff");
        assert!(!diff.hunks.is_empty());
        let has_word_diff = diff.hunks.iter().any(|h| {
            h.lines
                .iter()
                .any(|l| matches!(l, DiffLine::WordDiff { .. }))
        });
        assert!(has_word_diff, "should have WordDiff lines");
    }

    #[test]
    fn whitespace_ignore_blank_lines_filters_blank() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "line1\n\n\nline2", "first");
        let oid2 = temp.commit_file("a.txt", "line1\nline2", "second");
        let repo = GixRepository::open(temp.path()).expect("open");
        let diff_normal = repo
            .file_diff(
                Some(oid1),
                oid2,
                std::path::Path::new("a.txt"),
                DiffMode::Normal,
                WhitespaceMode::None,
            )
            .expect("file_diff");
        let diff_filtered = repo
            .file_diff(
                Some(oid1),
                oid2,
                std::path::Path::new("a.txt"),
                DiffMode::Normal,
                WhitespaceMode::IgnoreBlankLines,
            )
            .expect("file_diff");
        assert!(
            !diff_normal.hunks.is_empty(),
            "normal mode should show blank line changes"
        );
        assert!(
            diff_filtered.hunks.is_empty(),
            "ignore-blank-lines should filter blank line changes"
        );
    }

    #[test]
    fn reflog_returns_entries_for_head() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        temp.commit_file("b.txt", "world", "second commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let entries = repo.reflog(None).expect("reflog");
        assert!(entries.len() >= 2, "should have at least 2 reflog entries");
        assert_eq!(entries[0].ref_name, "HEAD");
        assert!(!entries[0].message.is_empty() || !entries[0].oid.to_hex().is_empty());
    }

    #[test]
    fn reflog_for_named_ref() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        run_git(temp.path(), &["checkout", "-b", "feature"]);
        temp.commit_file("b.txt", "world", "on feature");
        let repo = GixRepository::open(temp.path()).expect("open");
        let entries = repo.reflog(Some("refs/heads/feature")).expect("reflog");
        assert!(
            entries.len() >= 1,
            "should have at least 1 reflog entry for feature branch"
        );
    }

    #[test]
    fn reflog_empty_for_unborn() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        run_git(dir.path(), &["init"]);
        run_git(dir.path(), &["config", "user.name", "Test"]);
        run_git(dir.path(), &["config", "user.email", "test@test.com"]);
        let repo = GixRepository::open(dir.path()).expect("open");
        let entries = repo.reflog(None).expect("reflog");
        assert!(entries.is_empty(), "unborn repo should have no reflog");
    }

    #[test]
    fn stash_list_empty_when_no_stashes() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let stashes = repo.stash_list().expect("stash_list");
        assert!(stashes.is_empty());
    }

    #[test]
    fn stash_list_returns_entries() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        std::fs::write(temp.dir.path().join("a.txt"), "modified").expect("write");
        run_git(temp.path(), &["stash", "--include-untracked"]);
        let repo = GixRepository::open(temp.path()).expect("open");
        let stashes = repo.stash_list().expect("stash_list");
        assert_eq!(stashes.len(), 1);
        assert_eq!(stashes[0].index, 0);
        assert!(!stashes[0].message.is_empty());
        assert!(!stashes[0].file_summary.is_empty());
    }

    #[test]
    fn stash_list_multiple_stashes() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        std::fs::write(temp.dir.path().join("a.txt"), "mod1").expect("write");
        run_git(temp.path(), &["stash"]);
        std::fs::write(temp.dir.path().join("a.txt"), "mod2").expect("write");
        run_git(temp.path(), &["stash"]);
        let repo = GixRepository::open(temp.path()).expect("open");
        let stashes = repo.stash_list().expect("stash_list");
        assert_eq!(stashes.len(), 2);
        assert_eq!(stashes[0].index, 0);
        assert_eq!(stashes[1].index, 1);
    }

    #[test]
    fn stash_diff_returns_combined_diff() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        std::fs::write(temp.dir.path().join("a.txt"), "modified content").expect("write");
        run_git(temp.path(), &["stash"]);
        let repo = GixRepository::open(temp.path()).expect("open");
        let diff = repo.stash_diff(0).expect("stash_diff");
        assert!(
            !diff.hunks.is_empty() || diff.is_binary,
            "stash diff should have hunks"
        );
    }

    #[test]
    fn stash_split_diff_returns_staged_and_unstaged() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "hello", "first commit");
        std::fs::write(temp.dir.path().join("a.txt"), "modified").expect("write");
        run_git(temp.path(), &["stash"]);
        let repo = GixRepository::open(temp.path()).expect("open");
        let split = repo.stash_split_diff(0).expect("stash_split_diff");
        let total_hunks = split.staged.hunks.len() + split.unstaged.hunks.len();
        assert!(
            total_hunks > 0 || split.staged.is_binary || split.unstaged.is_binary,
            "split diff should have hunks"
        );
    }

    #[test]
    fn blame_returns_line_attributions() {
        let temp = TempRepo::new();
        temp.commit_file("a.txt", "line1\nline2\nline3", "first commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let blame = repo
            .blame(std::path::Path::new("a.txt"), None)
            .expect("blame");
        assert_eq!(blame.lines.len(), 3);
        assert_eq!(blame.lines[0].content.trim(), "line1");
        assert_eq!(blame.lines[1].content.trim(), "line2");
        assert_eq!(blame.lines[2].content.trim(), "line3");
        assert_eq!(blame.lines[0].author.name, "Test");
        assert_eq!(blame.lines[0].author.email, "test@test.com");
    }

    #[test]
    fn blame_at_commit() {
        let temp = TempRepo::new();
        let oid1 = temp.commit_file("a.txt", "original", "first commit");
        let _oid2 = temp.commit_file("a.txt", "modified", "second commit");
        let repo = GixRepository::open(temp.path()).expect("open");
        let blame = repo
            .blame(std::path::Path::new("a.txt"), Some(oid1))
            .expect("blame");
        assert_eq!(blame.lines.len(), 1);
        assert_eq!(blame.lines[0].content.trim(), "original");
    }
}
