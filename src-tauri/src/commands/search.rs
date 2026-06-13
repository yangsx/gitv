use gitv_git_core::models::Oid;
use gitv_git_core::repository::Repository;
use gitv_git_core::search::{MatchType, SearchResponse, SearchResult};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tracing::instrument;

use crate::state::AppState;

#[derive(Serialize, Clone)]
pub struct PatchSearchProgress {
    pub search_id: u64,
    pub checked: u64,
    pub total: u64,
    pub matches: Vec<SearchResult>,
}

#[derive(Serialize, Clone)]
pub struct PatchSearchComplete {
    pub search_id: u64,
    pub total_checked: u64,
}

#[derive(Serialize, Clone)]
pub struct PatchSearchError {
    pub search_id: u64,
    pub message: String,
}

#[tauri::command]
#[instrument(skip(state, app_handle, path), fields(command = "search_commits"))]
pub fn search_commits(
    app_handle: AppHandle,
    state: State<AppState>,
    path: String,
    query: gitv_git_core::search::SearchQuery,
) -> Result<SearchResponse, String> {
    let repo_path = PathBuf::from(&path);
    let outcome = state.search(&repo_path, &query)?;

    let results = outcome.results;

    if outcome.patch_candidates.is_empty() {
        return Ok(SearchResponse {
            results,
            patch_search_id: None,
            patch_search_total: None,
        });
    }

    let search_id = state.new_patch_search()?;
    let cancel_flag = state
        .get_cancel_flag_for(search_id)
        .ok_or("failed to get cancel flag")?;

    let repo = state.get_repo(&repo_path)?;
    let total = outcome.patch_candidates.len() as u64;
    let pattern = query.text.as_ref().cloned().unwrap_or_default();
    let use_regex = query.use_regex;

    std::thread::spawn(move || {
        process_patch_search(
            app_handle,
            repo,
            outcome.patch_candidates,
            pattern,
            use_regex,
            search_id,
            cancel_flag,
            total,
        );
    });

    Ok(SearchResponse {
        results,
        patch_search_id: Some(search_id),
        patch_search_total: Some(total),
    })
}

#[tauri::command]
pub fn cancel_patch_search(state: State<AppState>, search_id: u64) -> Result<(), String> {
    state.cancel_patch_search(search_id)
}

#[allow(clippy::too_many_arguments)]
fn process_patch_search(
    app_handle: AppHandle,
    repo: std::sync::Arc<gitv_git_core::gix_repo::GixRepository>,
    candidates: Vec<Oid>,
    pattern: String,
    use_regex: bool,
    search_id: u64,
    cancel_flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    total: u64,
) {
    use std::sync::atomic::Ordering;

    let compiled_regex: Option<regex::Regex> = if use_regex {
        Some(match regex::Regex::new(&pattern) {
            Ok(re) => re,
            Err(e) => {
                let _ = app_handle.emit(
                    "patch-search-progress",
                    PatchSearchError {
                        search_id,
                        message: format!("Invalid regex: {e}"),
                    },
                );
                if let Some(state) = app_handle.try_state::<AppState>() {
                    state.finish_patch_search(search_id);
                }
                return;
            }
        })
    } else {
        None
    };

    let batch_size = 20usize;
    let mut checked = 0u64;

    for chunk in candidates.chunks(batch_size) {
        if cancel_flag.load(Ordering::SeqCst) {
            break;
        }

        let matches: Vec<SearchResult> = chunk
            .iter()
            .filter_map(|oid| {
                match repo.commit_patch_matches(oid, &pattern, compiled_regex.as_ref()) {
                    Ok(locs) if !locs.is_empty() => Some(SearchResult {
                        commit_oid: *oid,
                        match_type: MatchType::Patch,
                        highlights: Vec::new(),
                        patch_matches: locs,
                    }),
                    _ => None,
                }
            })
            .collect();

        checked += chunk.len() as u64;

        let _ = app_handle.emit(
            "patch-search-progress",
            PatchSearchProgress {
                search_id,
                checked,
                total,
                matches,
            },
        );
    }

    let _ = app_handle.emit(
        "patch-search-progress",
        PatchSearchComplete {
            search_id,
            total_checked: checked,
        },
    );

    if let Some(state) = app_handle.try_state::<AppState>() {
        state.finish_patch_search(search_id);
    }
}
