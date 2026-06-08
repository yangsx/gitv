use crate::state::AppState;
use gitv_git_core::cache::RepositoryCache;
use gitv_git_core::graph::{
    GraphCalculator, GraphColorMode, GraphOptions, GraphOrientation, GraphPalette,
};
use gitv_git_core::models::{
    CachedRepoData, CommitInfo, Oid, Ref, RepositoryInfo, WorkingChangesDiff,
};
use gitv_git_core::repository::Repository;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::State;
use tracing::instrument;

fn get_ref_snapshot(repo: &dyn Repository) -> HashMap<String, Oid> {
    let refs = match repo.refs() {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("failed to load refs for cache snapshot: {e}");
            return HashMap::new();
        }
    };
    refs.iter()
        .filter_map(|r| match r {
            Ref::Branch(b) => Some((format!("refs/heads/{}", b.name), b.oid)),
            Ref::Tag(t) => Some((format!("refs/tags/{}", t.name), t.oid)),
            Ref::Remote(r) => Some((format!("refs/remotes/{}/{}", r.remote, r.name), r.oid)),
            Ref::Head => None,
        })
        .collect()
}

fn load_commits(repo: &dyn Repository, repo_path: &Path) -> Result<Vec<CommitInfo>, String> {
    let cache = match RepositoryCache::open(repo_path) {
        Ok(c) => Some(c),
        Err(e) => {
            tracing::warn!("failed to open repo cache: {e}");
            None
        }
    };
    if let Some(ref cache) = cache
        && let Ok(Some(data)) = cache.load()
    {
        let snapshot = get_ref_snapshot(repo);
        if data.ref_snapshot == snapshot && !data.commit_summaries.is_empty() {
            return Ok(data
                .commit_summaries
                .into_iter()
                .map(CommitInfo::from)
                .collect());
        }
    }
    let commits = repo
        .commits(None)
        .map_err(|e| format!("failed to load commits: {e}"))?;
    if let Some(ref cache) = cache {
        let snapshot = get_ref_snapshot(repo);
        let data = CachedRepoData::from_commits(&commits, snapshot);
        let _ = cache.store(&data);
    }
    Ok(commits)
}

#[derive(Serialize)]
pub struct LoadTiming {
    pub load_commits_ms: f64,
    pub graph_calc_ms: f64,
    pub refs_ms: f64,
    pub working_changes_ms: f64,
    pub total_ms: f64,
}

#[derive(Serialize)]
pub struct InitialData {
    pub repo_info: RepositoryInfo,
    pub commits: Vec<CommitInfo>,
    pub graph_layout: gitv_git_core::graph::GraphLayout,
    pub refs: Vec<Ref>,
    pub working_changes: Option<WorkingChangesDiff>,
    pub timing: LoadTiming,
    pub warnings: Vec<String>,
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_initial_data"))]
pub fn get_initial_data(
    state: State<'_, AppState>,
    path: String,
    hide_merges: Option<bool>,
    orientation: Option<String>,
    color_mode: Option<String>,
    palette: Option<String>,
) -> Result<InitialData, String> {
    let start = Instant::now();
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let repo_info = repo.info().map_err(|e| e.to_string())?;

    let t0 = Instant::now();
    let commits =
        load_commits(&*repo, &repo_path).map_err(|e| format!("failed to load commits: {e}"))?;
    let load_commits_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let mut refs_map: HashMap<Oid, Vec<Ref>> = HashMap::new();
    for c in &commits {
        if !c.refs.is_empty() {
            refs_map.insert(c.oid, c.refs.clone());
        }
    }

    let orientation = match orientation.as_deref() {
        Some("bottom-to-top") => GraphOrientation::BottomToTop,
        _ => GraphOrientation::TopToBottom,
    };
    let color_mode = match color_mode.as_deref() {
        Some("by-author") => GraphColorMode::ByAuthor,
        _ => GraphColorMode::ByBranch,
    };
    let graph_palette = match palette.as_deref() {
        Some("deuteranopia") => GraphPalette::DeuteranopiaSafe,
        Some("protanopia") => GraphPalette::ProtanopiaSafe,
        Some("tritanopia") => GraphPalette::TritanopiaSafe,
        _ => GraphPalette::Default,
    };

    let options = GraphOptions {
        hide_merges: hide_merges.unwrap_or(false),
        orientation,
        color_mode,
        palette: graph_palette,
    };

    let mut warnings: Vec<String> = Vec::new();

    let stashes = match repo.stash_list() {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("failed to load stashes: {e}");
            tracing::warn!("{msg}");
            warnings.push(msg);
            Vec::new()
        }
    };

    let t1 = Instant::now();
    let graph_layout = {
        let calc = GraphCalculator::new(commits.clone(), refs_map, stashes, options);
        calc.calculate_layout()
    };
    let graph_calc_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let t2 = Instant::now();
    let refs = match repo.refs() {
        Ok(r) => r,
        Err(e) => {
            let msg = format!("failed to load refs: {e}");
            tracing::warn!("{msg}");
            warnings.push(msg);
            Vec::new()
        }
    };
    let refs_ms = t2.elapsed().as_secs_f64() * 1000.0;

    let t3 = Instant::now();
    let working_changes = repo.working_changes_diff().ok();
    let working_changes_ms = t3.elapsed().as_secs_f64() * 1000.0;

    let total_ms = start.elapsed().as_secs_f64() * 1000.0;

    Ok(InitialData {
        repo_info,
        commits,
        graph_layout,
        refs,
        working_changes,
        timing: LoadTiming {
            load_commits_ms,
            graph_calc_ms,
            refs_ms,
            working_changes_ms,
            total_ms,
        },
        warnings,
    })
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_graph_layout"))]
pub fn get_graph_layout(
    state: State<'_, AppState>,
    path: String,
    hide_merges: Option<bool>,
    orientation: Option<String>,
    color_mode: Option<String>,
    palette: Option<String>,
    focus_branch_oid: Option<String>,
) -> Result<gitv_git_core::graph::GraphLayout, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let commits = load_commits(&*repo, &repo_path)?;

    let mut refs_map: HashMap<Oid, Vec<Ref>> = HashMap::new();
    for c in &commits {
        if !c.refs.is_empty() {
            refs_map.insert(c.oid, c.refs.clone());
        }
    }

    let orientation = match orientation.as_deref() {
        Some("bottom-to-top") => GraphOrientation::BottomToTop,
        _ => GraphOrientation::TopToBottom,
    };

    let color_mode = match color_mode.as_deref() {
        Some("by-author") => GraphColorMode::ByAuthor,
        _ => GraphColorMode::ByBranch,
    };

    let graph_palette = match palette.as_deref() {
        Some("deuteranopia") => GraphPalette::DeuteranopiaSafe,
        Some("protanopia") => GraphPalette::ProtanopiaSafe,
        Some("tritanopia") => GraphPalette::TritanopiaSafe,
        _ => GraphPalette::Default,
    };

    let options = GraphOptions {
        hide_merges: hide_merges.unwrap_or(false),
        orientation,
        color_mode,
        palette: graph_palette,
    };

    let stashes = repo.stash_list().map_err(|e| e.to_string())?;
    let calc = GraphCalculator::new(commits, refs_map, stashes, options);
    let mut layout = calc.calculate_layout();

    if let Some(ref oid_hex) = focus_branch_oid
        && let Ok(focus_oid) = Oid::from_hex(oid_hex)
    {
        let ancestors = calc.get_ancestor_oids(&focus_oid);
        GraphCalculator::apply_dimming(&mut layout, Some(focus_oid), Some(&ancestors));
    }

    Ok(layout)
}
