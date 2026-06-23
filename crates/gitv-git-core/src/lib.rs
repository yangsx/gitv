pub mod cache;
pub mod error;
pub mod gix_repo;
pub mod graph;
pub mod models;
pub mod repository;
pub mod search;
pub mod stream;

/// Maximum number of diff lines returned per file before truncation.
pub const DIFF_LINE_LIMIT: usize = 10_000;

/// Number of bytes scanned for null bytes to classify a file as binary.
pub const BINARY_SCAN_SIZE: usize = 8192;

/// Maximum number of characters stored for a patch-search match.
pub const SEARCH_MATCH_MAX_LEN: usize = 200;

/// In-memory object cache size for gix (gitoxide) repositories, in bytes.
pub const OBJECT_CACHE_SIZE: usize = 10 * 1024 * 1024;
