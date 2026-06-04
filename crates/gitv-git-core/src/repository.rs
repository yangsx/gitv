use crate::error::{DiffError, GitError};
use crate::gix_repo::GixRepository;
use crate::models::*;
use std::path::Path;

pub trait Repository {
    fn info(&self) -> Result<RepositoryInfo, GitError>;
    fn commits(&self, max_count: Option<usize>) -> Result<Vec<CommitInfo>, GitError>;
    fn commit(&self, oid: Oid) -> Result<CommitDetails, GitError>;
    fn refs(&self) -> Result<Vec<Ref>, GitError>;
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
        self.file_diff_limited(from, to, path, mode, whitespace, Some(10_000))
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
    fn file_history(
        &self,
        path: &std::path::Path,
        max_count: Option<usize>,
    ) -> Result<Vec<FileHistoryEntry>, GitError>;
}

pub fn open(path: &Path) -> Result<Box<dyn Repository>, GitError> {
    let repo = GixRepository::open(path)?;
    Ok(Box::new(repo))
}
