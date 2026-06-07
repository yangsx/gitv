use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::CacheError;
use crate::models::*;

const CACHE_VERSION: u32 = 1;
const CACHE_FILENAME: &str = "repo-cache.bin";

pub struct RepositoryCache {
    cache_dir: PathBuf,
}

impl RepositoryCache {
    pub fn open(repo_path: &Path) -> Result<Self, CacheError> {
        let cache_root = dirs_cache_root();
        let repo_hash = cache_key_for_path(repo_path);
        let cache_dir = cache_root.join(repo_hash);
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    pub fn load(&self) -> Result<Option<CachedRepoData>, CacheError> {
        let path = self.cache_dir.join(CACHE_FILENAME);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path)?;
        let data: CachedRepoData =
            postcard::from_bytes(&bytes).map_err(|e| CacheError::Serialization(e.to_string()))?;
        if data.version != CACHE_VERSION {
            return Err(CacheError::VersionMismatch {
                expected: CACHE_VERSION,
                found: data.version,
            });
        }
        Ok(Some(data))
    }

    pub fn store(&self, data: &CachedRepoData) -> Result<(), CacheError> {
        let path = self.cache_dir.join(CACHE_FILENAME);
        let tmp_path = self.cache_dir.join(format!("{CACHE_FILENAME}.tmp"));
        let bytes =
            postcard::to_allocvec(data).map_err(|e| CacheError::Serialization(e.to_string()))?;
        std::fs::write(&tmp_path, bytes)?;
        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }
}

fn dirs_cache_root() -> PathBuf {
    let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("gitv")
}

fn cache_key_for_path(path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

impl CachedRepoData {
    pub fn new(ref_snapshot: HashMap<String, Oid>) -> Self {
        Self {
            ref_snapshot,
            commit_summaries: Vec::new(),
            version: CACHE_VERSION,
        }
    }

    pub fn from_commits(commits: &[CommitInfo], ref_snapshot: HashMap<String, Oid>) -> Self {
        Self {
            ref_snapshot,
            commit_summaries: commits
                .iter()
                .map(|c| CachedCommitSummary {
                    oid: c.oid,
                    summary: c.summary.clone(),
                    author: c.author.clone(),
                    author_time: c.author_time,
                    parent_oids: c.parent_oids.clone(),
                    refs: c.refs.clone(),
                })
                .collect(),
            version: CACHE_VERSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn test_cache_dir() -> PathBuf {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        path
    }

    #[test]
    fn open_creates_cache_dir() {
        let repo_path = PathBuf::from("/tmp/some/repo");
        let cache = RepositoryCache::open(&repo_path);
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert!(cache.cache_dir.exists());
        let _ = std::fs::remove_dir_all(cache.cache_dir.parent().unwrap().parent().unwrap());
    }

    #[test]
    fn load_returns_none_for_new_repo() {
        let cache = RepositoryCache {
            cache_dir: test_cache_dir(),
        };
        let result = cache.load().expect("load");
        assert!(result.is_none());
    }

    #[test]
    fn store_and_load_roundtrip() {
        let dir = test_cache_dir();
        let cache = RepositoryCache { cache_dir: dir };
        let mut ref_snapshot = HashMap::new();
        ref_snapshot.insert(
            "refs/heads/main".to_string(),
            Oid::from_hex("0123456789abcdeffedcba987654321000000001").unwrap(),
        );
        let data = CachedRepoData {
            ref_snapshot: ref_snapshot.clone(),
            commit_summaries: vec![CachedCommitSummary {
                oid: Oid::from_hex("0123456789abcdeffedcba987654321000000001").unwrap(),
                summary: "test commit".to_string(),
                author: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                author_time: Utc::now(),
                parent_oids: vec![],
                refs: vec![],
            }],
            version: CACHE_VERSION,
        };
        cache.store(&data).expect("store");
        let loaded = cache.load().expect("load").expect("some data");
        assert_eq!(loaded.ref_snapshot, ref_snapshot);
        assert_eq!(loaded.commit_summaries.len(), 1);
        assert_eq!(loaded.commit_summaries[0].summary, "test commit");
        assert_eq!(loaded.version, CACHE_VERSION);
    }

    #[test]
    fn version_mismatch_returns_error() {
        let dir = test_cache_dir();
        let cache = RepositoryCache { cache_dir: dir };
        let data = CachedRepoData {
            ref_snapshot: HashMap::new(),
            commit_summaries: vec![],
            version: 999,
        };
        cache.store(&data).expect("store");
        let result = cache.load();
        assert!(matches!(result, Err(CacheError::VersionMismatch { .. })));
    }
}
