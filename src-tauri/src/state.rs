use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::{CommitInfo, Oid};
use gitv_git_core::repository::Repository;
use gitv_git_core::search::{SearchEngine, SearchQuery};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

pub struct AppState {
    search_engines: Mutex<HashMap<PathBuf, SearchEngine>>,
    repo_cache: Mutex<HashMap<PathBuf, Arc<GixRepository>>>,
    patch_searches: Mutex<HashMap<u64, Arc<AtomicBool>>>,
    next_patch_search_id: AtomicU64,
    commit_store: Mutex<HashMap<PathBuf, Arc<Vec<CommitInfo>>>>,
    commit_store_order: Mutex<VecDeque<PathBuf>>,
    search_engine_order: Mutex<VecDeque<PathBuf>>,
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

    pub fn search(&self, repo_path: &Path, query: &SearchQuery) -> Result<SearchOutcome, String> {
        let (results, all_oids) = {
            let mut cache = self.search_engines.lock().map_err(|e| e.to_string())?;
            let mut order = self.search_engine_order.lock().map_err(|e| e.to_string())?;

            if !cache.contains_key(repo_path) {
                // Evict LRU if at capacity
                if cache.len() >= MAX_SEARCH_ENGINE_REPOS
                    && let Some(oldest) = order.pop_front()
                {
                    cache.remove(&oldest);
                }
                let repo = self.get_repo(repo_path)?;
                let commits = repo.commits(None, &[]).map_err(|e| e.to_string())?;
                let engine = SearchEngine::new(commits);
                cache.insert(repo_path.to_path_buf(), engine);
                // Update access order
                order.retain(|p| p != repo_path);
                order.push_back(repo_path.to_path_buf());
            } else {
                // Update access order for existing entry
                order.retain(|p| p != repo_path);
                order.push_back(repo_path.to_path_buf());
            }

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

    pub fn get_repo(&self, path: &Path) -> Result<Arc<GixRepository>, String> {
        let mut cache = self.repo_cache.lock().map_err(|e| e.to_string())?;
        if let Some(repo) = cache.get(path) {
            return Ok(Arc::clone(repo));
        }
        let repo = GixRepository::open(path).map_err(|e| e.to_string())?;
        let repo = Arc::new(repo);
        cache.insert(path.to_path_buf(), Arc::clone(&repo));
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
