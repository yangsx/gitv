use crate::error::{DiffError, GitError};
use crate::gix_repo::GixRepository;
use crate::models::*;
use std::path::Path;

pub trait Repository {
    fn info(&self) -> Result<RepositoryInfo, GitError>;
    fn commits(
        &self,
        max_count: Option<usize>,
        extra_tips: &[Oid],
    ) -> Result<Vec<CommitInfo>, GitError>;
    fn commit(&self, oid: Oid) -> Result<CommitDetails, GitError> {
        self.commit_details(oid, false)
    }

    /// Compute per-file line counts (additions/deletions) for a commit.
    /// This reads blob contents and is expensive — call lazily.
    fn commit_file_counts(&self, oid: Oid) -> Result<Vec<FileLineStats>, GitError> {
        let details = self.commit_details(oid, true)?;
        Ok(details
            .changed_files
            .into_iter()
            .map(|f| FileLineStats {
                path: f.path,
                additions: f.additions,
                deletions: f.deletions,
            })
            .collect())
    }

    /// Load commit details with an optional line-count pass.
    /// When `include_counts` is true, reads blob contents for each changed
    /// file to compute additions/deletions (expensive — use only when needed).
    fn commit_details(&self, oid: Oid, include_counts: bool) -> Result<CommitDetails, GitError>;
    fn refs(&self) -> Result<Vec<Ref>, GitError>;

    /// Lightweight ref enumeration — returns ref name → OID mapping without
    /// computing `is_merged` (no HEAD ancestor walk).  Use this for cache
    /// validation snapshots.
    fn ref_snapshot(&self) -> Result<std::collections::HashMap<String, Oid>, GitError>;

    /// Same as [`refs`](Self::refs) but accepts a precomputed HEAD ancestor
    /// set, skipping the expensive internal walk.
    fn refs_with_ancestors(
        &self,
        head_ancestors: &std::collections::HashSet<Oid>,
    ) -> Result<Vec<Ref>, GitError>;
    fn stash_list(&self) -> Result<Vec<StashEntry>, GitError>;
    fn stash_diff(&self, stash_index: usize) -> Result<FileDiff, DiffError>;
    fn stash_split_diff(&self, stash_index: usize) -> Result<StashSplitDiff, DiffError>;
    fn reflog(&self, ref_name: Option<&str>) -> Result<Vec<ReflogEntry>, GitError>;
    fn blame(&self, path: &Path, at_commit: Option<Oid>) -> Result<Blame, GitError>;
    fn file_tree(&self, at_commit: Option<Oid>) -> Result<FileTreeNode, GitError>;
    fn is_bare(&self) -> bool;
    fn diff_summary(
        &self,
        from: Option<Oid>,
        to: Oid,
        whitespace: WhitespaceMode,
    ) -> Result<DiffSummary, DiffError>;
    fn file_diff(
        &self,
        from: Option<Oid>,
        to: Oid,
        path: &std::path::Path,
        mode: DiffMode,
        whitespace: WhitespaceMode,
    ) -> Result<FileDiff, DiffError> {
        self.file_diff_limited(
            from,
            to,
            path,
            mode,
            whitespace,
            Some(crate::DIFF_LINE_LIMIT),
        )
    }

    fn file_diff_limited(
        &self,
        from: Option<Oid>,
        to: Oid,
        path: &std::path::Path,
        mode: DiffMode,
        whitespace: WhitespaceMode,
        line_limit: Option<usize>,
    ) -> Result<FileDiff, DiffError>;
    fn combined_file_diff(
        &self,
        merge_oid: Oid,
        path: &std::path::Path,
        mode: DiffMode,
        whitespace: WhitespaceMode,
        line_limit: Option<usize>,
    ) -> Result<FileDiff, DiffError> {
        self.file_diff_limited(None, merge_oid, path, mode, whitespace, line_limit)
    }
    fn file_history(
        &self,
        path: &std::path::Path,
        max_count: Option<usize>,
    ) -> Result<Vec<FileHistoryEntry>, GitError>;
    fn blob_content(&self, oid: Oid, path: &Path) -> Result<String, GitError>;
    fn blob_content_staged(&self, path: &Path) -> Result<String, GitError>;
    fn blob_content_worktree(&self, path: &Path) -> Result<String, GitError>;
    fn working_changes_diff(&self) -> Result<WorkingChangesDiff, GitError>;
    fn working_changes_file_diffs(
        &self,
        staged: bool,
        mode: DiffMode,
        whitespace: WhitespaceMode,
    ) -> Result<Vec<FileDiff>, DiffError>;
    fn working_changes_combined_diff(
        &self,
        mode: DiffMode,
        whitespace: WhitespaceMode,
    ) -> Result<Vec<FileDiff>, DiffError>;
    fn commit_patch_matches(
        &self,
        oid: &Oid,
        pattern: &str,
        regex: Option<&regex::Regex>,
    ) -> Result<Vec<crate::search::PatchMatchLocation>, GitError>;
    /// For merge commits (2+ parents): returns files that differ from ALL parents
    /// with a `diff_parent` hint per file.  For non-merge commits delegates to
    /// [`commit_details`](Self::commit_details).
    fn combined_diff(&self, oid: Oid, include_counts: bool) -> Result<CommitDetails, GitError>;
}

pub fn open(path: &Path) -> Result<Box<dyn Repository>, GitError> {
    let repo = GixRepository::open(path)?;
    Ok(Box::new(repo))
}
