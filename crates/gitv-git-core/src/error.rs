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
    #[error("gix error: {0}")]
    Gix(String),
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

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },
}

#[derive(Error, Debug)]
pub enum OidError {
    #[error("invalid hex length: expected 40 characters, got {0}")]
    InvalidLength(usize),
    #[error("invalid hex character at position {0}")]
    InvalidChar(usize),
}
