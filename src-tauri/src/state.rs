use gitv_git_core::gix_repo::{
    GixRepository, is_remote_url, remote_cache_root, remote_cache_subdir,
};
use gitv_git_core::models::{CommitInfo, Oid};
use gitv_git_core::repository::Repository;
use gitv_git_core::search::{SearchEngine, SearchQuery};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tracing::instrument;

pub fn resolve_repo_key(path_or_url: &str) -> PathBuf {
    if is_remote_url(path_or_url) {
        remote_cache_root().join(remote_cache_subdir(path_or_url))
    } else {
        PathBuf::from(shellexpand::tilde(path_or_url).as_ref())
    }
}

pub struct AppState {
    search_engines: Mutex<HashMap<PathBuf, SearchEngine>>,
    repo_cache: Mutex<HashMap<PathBuf, Arc<GixRepository>>>,
    patch_searches: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    next_patch_search_id: AtomicU64,
    commit_store: Mutex<HashMap<PathBuf, Arc<Vec<CommitInfo>>>>,
    commit_store_order: Mutex<VecDeque<PathBuf>>,
    search_engine_order: Mutex<VecDeque<PathBuf>>,
    engine_build_locks: Mutex<HashMap<PathBuf, Arc<std::sync::Mutex<()>>>>,
}

/// Maximum number of repos whose commits are retained in memory.
const MAX_COMMIT_STORE_REPOS: usize = 8;

/// Maximum number of search engines retained in memory.
const MAX_SEARCH_ENGINE_REPOS: usize = 4;

impl AppState {
    pub fn new() -> Self {
        Self {
            search_engines: Mutex::new(HashMap::new()),
            repo_cache: Mutex::new(HashMap::new()),
            patch_searches: Mutex::new(HashMap::new()),
            next_patch_search_id: AtomicU64::new(1),
            commit_store: Mutex::new(HashMap::new()),
            commit_store_order: Mutex::new(VecDeque::new()),
            search_engine_order: Mutex::new(VecDeque::new()),
            engine_build_locks: Mutex::new(HashMap::new()),
        }
    }

    pub fn store_commits(&self, repo_path: &Path, commits: Vec<CommitInfo>) {
        if let (Ok(mut store), Ok(mut order)) =
            (self.commit_store.lock(), self.commit_store_order.lock())
        {
            // Remove old entry from order if exists
            order.retain(|p| p != repo_path);
            // Evict LRU if at capacity
            if store.len() >= MAX_COMMIT_STORE_REPOS
                && let Some(oldest) = order.pop_front()
            {
                store.remove(&oldest);
            }
            store.insert(repo_path.to_path_buf(), Arc::new(commits));
            order.push_back(repo_path.to_path_buf());
        }
    }

    pub fn get_commit_batch(
        &self,
        repo_path: &Path,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, String> {
        let (store, mut order) = (
            self.commit_store.lock().map_err(|e| e.to_string())?,
            self.commit_store_order.lock().map_err(|e| e.to_string())?,
        );
        let commits = store
            .get(repo_path)
            .ok_or("commits not loaded for this repo")?;
        let end = offset.saturating_add(limit).min(commits.len());
        // Update LRU order — mark this repo as recently used
        order.retain(|p| p != repo_path);
        order.push_back(repo_path.to_path_buf());
        Ok(commits[offset..end].to_vec())
    }

    pub fn get_commits_by_oids(
        &self,
        repo_path: &Path,
        oids: &HashSet<Oid>,
    ) -> Result<Vec<CommitInfo>, String> {
        let store = self.commit_store.lock().map_err(|e| e.to_string())?;
        let commits = store
            .get(repo_path)
            .ok_or("commits not loaded for this repo")?;
        Ok(commits
            .iter()
            .filter(|c| oids.contains(&c.oid))
            .cloned()
            .collect())
    }

    #[instrument(level = "debug", skip(self), fields(repo = %repo_path.display()))]
    pub fn search(&self, repo_path: &Path, query: &SearchQuery) -> Result<SearchOutcome, String> {
        self.ensure_engine(repo_path)?;

        let (results, all_oids) = {
            let cache = self.search_engines.lock().map_err(|e| e.to_string())?;
            let mut order = self.search_engine_order.lock().map_err(|e| e.to_string())?;
            order.retain(|p| p != repo_path);
            order.push_back(repo_path.to_path_buf());

            let engine = cache.get(repo_path).ok_or("engine not found")?;
            let results = engine.search(query).map_err(|e| e.to_string())?;
            let all_oids = engine.commit_oids();
            (results, all_oids)
        };

        if !query.search_patch {
            return Ok(SearchOutcome {
                results,
                patch_candidates: Vec::new(),
            });
        }

        let Some(_pattern) = query.text.as_ref().filter(|t| !t.is_empty()) else {
            return Ok(SearchOutcome {
                results,
                patch_candidates: Vec::new(),
            });
        };

        let already_matched: HashSet<Oid> = results.iter().map(|r| r.commit_oid).collect();
        let candidates: Vec<Oid> = all_oids
            .into_iter()
            .filter(|oid| !already_matched.contains(oid))
            .collect();

        Ok(SearchOutcome {
            results,
            patch_candidates: candidates,
        })
    }

    fn ensure_engine(&self, repo_path: &Path) -> Result<(), String> {
        // Fast path: engine already cached
        {
            let cache = self.search_engines.lock().map_err(|e| e.to_string())?;
            if cache.contains_key(repo_path) {
                return Ok(());
            }
        }

        // Serialize engine builds per repo: concurrent typeahead searches
        // must not each build the engine from scratch.
        let build_lock = {
            let mut locks = self.engine_build_locks.lock().map_err(|e| e.to_string())?;
            locks
                .entry(repo_path.to_path_buf())
                .or_insert_with(|| Arc::new(std::sync::Mutex::new(())))
                .clone()
        };
        let _guard = build_lock.lock().map_err(|e| e.to_string())?;

        // Double-check: another thread may have built the engine while we waited
        {
            let cache = self.search_engines.lock().map_err(|e| e.to_string())?;
            if cache.contains_key(repo_path) {
                return Ok(());
            }
        }

        // Evict LRU if at capacity
        {
            let mut cache = self.search_engines.lock().map_err(|e| e.to_string())?;
            let mut order = self.search_engine_order.lock().map_err(|e| e.to_string())?;
            if cache.len() >= MAX_SEARCH_ENGINE_REPOS
                && let Some(oldest) = order.pop_front()
            {
                cache.remove(&oldest);
            }
        }

        let _build_span = tracing::debug_span!("build_search_engine").entered();
        let repo = self.get_repo(repo_path)?;
        let commits = repo.commits(None, &[]).map_err(|e| e.to_string())?;
        let engine = SearchEngine::new(commits);

        let mut cache = self.search_engines.lock().map_err(|e| e.to_string())?;
        let mut order = self.search_engine_order.lock().map_err(|e| e.to_string())?;
        cache.insert(repo_path.to_path_buf(), engine);
        order.retain(|p| p != repo_path);
        order.push_back(repo_path.to_path_buf());

        Ok(())
    }

    pub fn get_repo(&self, path: &Path) -> Result<Arc<GixRepository>, String> {
        let mut cache = self.repo_cache.lock().map_err(|e| e.to_string())?;
        if let Some(repo) = cache.get(path) {
            return Ok(Arc::clone(repo));
        }
        let repo = GixRepository::open(path)
            .or_else(|first_err| GixRepository::open_bare(path).map_err(|_| first_err))
            .map_err(|e| e.to_string())?;
        let repo = Arc::new(repo);
        cache.insert(path.to_path_buf(), Arc::clone(&repo));
        Ok(repo)
    }

    pub fn get_or_clone_remote(
        &self,
        url: &str,
        token: Option<&str>,
        force_refresh: bool,
    ) -> Result<Arc<GixRepository>, String> {
        let cache_root = remote_cache_root();
        let cache_path = cache_root.join(remote_cache_subdir(url));

        if !force_refresh {
            let cache = self.repo_cache.lock().map_err(|e| e.to_string())?;
            if let Some(repo) = cache.get(&cache_path) {
                return Ok(Arc::clone(repo));
            }
        } else {
            let mut cache = self.repo_cache.lock().map_err(|e| e.to_string())?;
            cache.remove(&cache_path);
        }

        let repo = GixRepository::open_remote_with_token(url, &cache_root, token)
            .map_err(|e| e.to_string())?;
        let repo = Arc::new(repo);
        let actual_path = repo.path().to_path_buf();
        let mut cache = self.repo_cache.lock().map_err(|e| e.to_string())?;
        cache.insert(actual_path, Arc::clone(&repo));
        cache.insert(cache_path, Arc::clone(&repo));
        Ok(repo)
    }

    pub fn new_patch_search(&self) -> Result<u64, String> {
        let id = self.next_patch_search_id.fetch_add(1, Ordering::SeqCst);
        let cancel_flag = Arc::new(AtomicBool::new(false));
        self.patch_searches
            .lock()
            .map_err(|e| e.to_string())?
            .insert(id, cancel_flag);
        Ok(id)
    }

    pub fn cancel_patch_search(&self, search_id: u64) -> Result<(), String> {
        let flag = self
            .patch_searches
            .lock()
            .map_err(|e| e.to_string())?
            .get(&search_id)
            .cloned();
        if let Some(f) = flag {
            f.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    pub fn finish_patch_search(&self, search_id: u64) {
        if let Ok(mut map) = self.patch_searches.lock() {
            map.remove(&search_id);
        }
    }

    pub fn get_cancel_flag_for(&self, search_id: u64) -> Option<Arc<AtomicBool>> {
        self.patch_searches.lock().ok()?.get(&search_id).cloned()
    }
}

pub struct SearchOutcome {
    pub results: Vec<gitv_git_core::search::SearchResult>,
    pub patch_candidates: Vec<Oid>,
}
