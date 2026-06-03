use crate::error::GitError;
use crate::models::*;
use std::path::Path;

pub trait Repository {
    fn info(&self) -> Result<RepositoryInfo, GitError>;
    fn commits(&self, max_count: Option<usize>) -> Result<Vec<CommitInfo>, GitError>;
    fn commit(&self, oid: Oid) -> Result<CommitDetails, GitError>;
    fn refs(&self) -> Result<Vec<Ref>, GitError>;
    fn stash_list(&self) -> Result<Vec<StashEntry>, GitError>;
    fn reflog(&self, ref_name: Option<&str>) -> Result<Vec<ReflogEntry>, GitError>;
    fn file_tree(&self, at_commit: Option<Oid>) -> Result<FileTreeNode, GitError>;
    fn is_bare(&self) -> bool;
}

pub fn open(path: &Path) -> Result<Box<dyn Repository>, GitError> {
    let git_dir = path.join(".git");
    if !git_dir.exists() {
        let bare_check = path.join("HEAD");
        if !bare_check.exists() {
            return Err(GitError::NotAGitRepository(path.display().to_string()));
        }
    }
    Err(GitError::NotFound(
        "repository backend not yet implemented".into(),
    ))
}
