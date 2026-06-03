use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("object not found: {0}")]
    ObjectNotFound(String),
    #[error("invalid object: {0}")]
    InvalidObject(String),
    #[error("reference not found: {0}")]
    RefNotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("repository not found: {0}")]
    NotFound(String),
    #[error("not a git repository: {0}")]
    NotAGitRepository(String),
    #[error("corrupted repository: {0}")]
    Corrupted(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("index not built")]
    IndexNotBuilt,
}

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("layout calculation failed: {0}")]
    LayoutFailed(String),
}
