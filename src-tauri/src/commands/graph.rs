use crate::state::AppState;
use gitv_git_core::cache::RepositoryCache;
use gitv_git_core::graph::{
    GraphCalculator, GraphColorMode, GraphOptions, GraphOrientation, GraphPalette,
};
use gitv_git_core::models::{
    Author, CachedRepoData, CommitInfo, Oid, Ref, RepositoryInfo, STAGED_OID, UNSTAGED_OID,
    WorkingChangesDiff,
};
use gitv_git_core::repository::Repository;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use tracing::instrument;

fn get_ref_snapshot(repo: &dyn Repository) -> HashMap<String, Oid> {
    repo.ref_snapshot().unwrap_or_default()
}

/// Compute the set of all ancestors reachable from HEAD using the
/// already-loaded commit graph.  This replaces the expensive
/// `repo.rev_walk([head]).all()` inside `refs()` with a fast BFS
/// through `parent_oids`.
fn compute_head_ancestors(commits: &[CommitInfo], head_oid: Option<&Oid>) -> HashSet<Oid> {
    let mut set = HashSet::new();
    let Some(head) = head_oid else {
        return set;
    };

    let oid_map: HashMap<&Oid, &CommitInfo> = commits.iter().map(|c| (&c.oid, c)).collect();

    let mut stack = vec![head];
    while let Some(oid) = stack.pop() {
        if !set.insert(*oid) {
            continue;
        }
        if let Some(commit) = oid_map.get(oid) {
            stack.extend(&commit.parent_oids);
        }
    }
    set
}

fn load_commits(
    repo: &dyn Repository,
    repo_path: &Path,
    extra_tips: &[Oid],
) -> Result<Vec<CommitInfo>, String> {
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
        .commits(None, extra_tips)
        .map_err(|e| format!("failed to load commits: {e}"))?;
    if let Some(ref cache) = cache {
        let snapshot = get_ref_snapshot(repo);
        let data = CachedRepoData::from_commits(&commits, snapshot);
        let _ = cache.store(&data);
    }
    Ok(commits)
}

pub fn build_refs_map(commits: &[CommitInfo]) -> HashMap<Oid, Vec<Ref>> {
    let mut refs_map: HashMap<Oid, Vec<Ref>> = HashMap::new();
    for c in commits {
        if !c.refs.is_empty() {
            refs_map.insert(c.oid, c.refs.clone());
        }
    }
    refs_map
}

/// Create virtual CommitInfo nodes for staged/unstaged working changes.
///
/// These are injected *before* graph layout computation so the column
/// assignment algorithm treats them as regular commits: they get col 0
/// (mainline), and other branches are pushed to higher columns.
///
/// When both exist, the chain is `unstaged → staged → HEAD`, matching
/// the conceptual layering (working tree = index + unstaged; index = HEAD + staged).
pub fn make_virtual_commits(
    head_oid: Option<&Oid>,
    working_changes: Option<&WorkingChangesDiff>,
) -> Vec<CommitInfo> {
    let Some(wc) = working_changes else {
        return vec![];
    };
    let has_staged = !wc.staged.is_empty();
    let has_unstaged = !wc.unstaged.is_empty();
    if !has_staged && !has_unstaged {
        return vec![];
    }

    let empty_author = Author {
        name: String::new(),
        email: String::new(),
    };
    let now = chrono::Utc::now();
    let head = head_oid.cloned().unwrap_or(Oid::from_bytes([0u8; 20]));

    let mut virtuals = Vec::with_capacity(2);

    if has_unstaged {
        let parent = if has_staged {
            vec![STAGED_OID]
        } else {
            vec![head]
        };
        virtuals.push(CommitInfo {
            oid: UNSTAGED_OID,
            short_oid: String::new(),
            message: String::new(),
            author: empty_author.clone(),
            committer: empty_author.clone(),
            author_time: now,
            commit_time: now,
            parent_oids: parent,
            refs: vec![],
        });
    }

    if has_staged {
        virtuals.push(CommitInfo {
            oid: STAGED_OID,
            short_oid: String::new(),
            message: String::new(),
            author: empty_author.clone(),
            committer: empty_author.clone(),
            author_time: now,
            commit_time: now,
            parent_oids: vec![head],
            refs: vec![],
        });
    }

    virtuals
}

/// Load commits from cache/repo, then prepend virtual working-changes commits.
/// Returns the augmented Vec plus the working changes diff (for the frontend).
fn load_commits_with_virtuals(
    repo: &dyn Repository,
    repo_path: &Path,
    stash_parent_tips: &[Oid],
    head_oid: Option<&Oid>,
) -> Result<(Vec<CommitInfo>, Option<WorkingChangesDiff>), String> {
    let commits = load_commits(repo, repo_path, stash_parent_tips)?;
    let working_changes = repo.working_changes_diff().ok();
    let virtuals = make_virtual_commits(head_oid, working_changes.as_ref());

    let mut all = virtuals;
    all.extend(commits);
    Ok((all, working_changes))
}

fn parse_graph_options(
    hide_merges: Option<bool>,
    orientation: Option<String>,
    color_mode: Option<String>,
    palette: Option<String>,
    arrow_gap_threshold: Option<usize>,
) -> GraphOptions {
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
    GraphOptions {
        hide_merges: hide_merges.unwrap_or(false),
        orientation,
        color_mode,
        palette: graph_palette,
        arrow_gap_threshold: arrow_gap_threshold.unwrap_or(100),
    }
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
    pub total_commit_count: usize,
    pub graph_layout: gitv_git_core::graph::GraphLayout,
    pub refs: Vec<Ref>,
    pub working_changes: Option<WorkingChangesDiff>,
    pub timing: LoadTiming,
    pub warnings: Vec<String>,
}

const INITIAL_COMMIT_LIMIT: usize = 1000;

struct InitialDataWork {
    repo_info: RepositoryInfo,
    graph_layout: gitv_git_core::graph::GraphLayout,
    all_commits: Vec<CommitInfo>,
    refs: Vec<Ref>,
    working_changes: Option<WorkingChangesDiff>,
    load_commits_ms: f64,
    graph_calc_ms: f64,
    refs_ms: f64,
    working_changes_ms: f64,
    total_ms: f64,
    warnings: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_initial_data"))]
pub async fn get_initial_data(
    state: State<'_, AppState>,
    path: String,
    hide_merges: Option<bool>,
    orientation: Option<String>,
    color_mode: Option<String>,
    palette: Option<String>,
    arrow_gap_threshold: Option<usize>,
) -> Result<InitialData, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let repo_for_blocking = Arc::clone(&repo);
    let repo_path_for_blocking = repo_path.clone();
    let options = parse_graph_options(
        hide_merges,
        orientation,
        color_mode,
        palette,
        arrow_gap_threshold,
    );

    let work = tauri::async_runtime::spawn_blocking(move || -> Result<InitialDataWork, String> {
        let start = Instant::now();
        let repo_info = repo_for_blocking.info().map_err(|e| e.to_string())?;
        let mut warnings: Vec<String> = Vec::new();

        let t0 = Instant::now();
        let stashes = match repo_for_blocking.stash_list() {
            Ok(s) => s,
            Err(e) => {
                let msg = format!("failed to load stashes: {e}");
                tracing::warn!("{msg}");
                warnings.push(msg);
                Vec::new()
            }
        };
        let stash_parent_tips: Vec<Oid> = stashes.iter().map(|s| s.parent_oid).collect();

        // Load commits + working changes together so virtual commits
        // (staged/unstaged) can be injected before graph layout computation.
        let (commits, working_changes) = load_commits_with_virtuals(
            &*repo_for_blocking,
            &repo_path_for_blocking,
            &stash_parent_tips,
            repo_info.head_commit.as_ref(),
        )
        .map_err(|e| format!("failed to load commits: {e}"))?;
        let load_commits_ms = t0.elapsed().as_secs_f64() * 1000.0;

        let head_ancestors = compute_head_ancestors(&commits, repo_info.head_commit.as_ref());
        let refs_map = build_refs_map(&commits);

        let t1 = Instant::now();
        let calc = GraphCalculator::new(commits, refs_map, stashes, options);
        let graph_layout = calc.calculate_layout();
        let mut all_commits = calc.into_commits();

        // Sort commits by layout row so that lazy-loaded batches
        // (get_commit_batch) correspond to sequential graph rows.
        let row_map: HashMap<Oid, usize> =
            graph_layout.nodes.iter().map(|n| (n.oid, n.row)).collect();
        all_commits.sort_by_key(|c| row_map.get(&c.oid).copied().unwrap_or(usize::MAX));

        let graph_calc_ms = t1.elapsed().as_secs_f64() * 1000.0;

        let t2 = Instant::now();
        let refs = match repo_for_blocking.refs_with_ancestors(&head_ancestors) {
            Ok(r) => r,
            Err(e) => {
                let msg = format!("failed to load refs: {e}");
                tracing::warn!("{msg}");
                warnings.push(msg);
                Vec::new()
            }
        };
        let refs_ms = t2.elapsed().as_secs_f64() * 1000.0;

        // Working changes already loaded above; timing is subsumed in load_commits_ms.
        let working_changes_ms = 0.0;

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;

        Ok(InitialDataWork {
            repo_info,
            graph_layout,
            all_commits,
            refs,
            working_changes,
            load_commits_ms,
            graph_calc_ms,
            refs_ms,
            working_changes_ms,
            total_ms,
            warnings,
        })
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))??;

    let total_commit_count = work.all_commits.len();
    state.store_commits(&repo_path, work.all_commits);
    let initial_commits = state.get_commit_batch(&repo_path, 0, INITIAL_COMMIT_LIMIT)?;

    Ok(InitialData {
        repo_info: work.repo_info,
        commits: initial_commits,
        total_commit_count,
        graph_layout: work.graph_layout,
        refs: work.refs,
        working_changes: work.working_changes,
        timing: LoadTiming {
            load_commits_ms: work.load_commits_ms,
            graph_calc_ms: work.graph_calc_ms,
            refs_ms: work.refs_ms,
            working_changes_ms: work.working_changes_ms,
            total_ms: work.total_ms,
        },
        warnings: work.warnings,
    })
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_commits_batch"))]
pub fn get_commits_batch(
    state: State<'_, AppState>,
    path: String,
    offset: usize,
    limit: usize,
) -> Result<Vec<CommitInfo>, String> {
    let repo_path = PathBuf::from(&path);
    state.get_commit_batch(&repo_path, offset, limit)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_graph_layout"))]
pub async fn get_graph_layout(
    state: State<'_, AppState>,
    path: String,
    hide_merges: Option<bool>,
    orientation: Option<String>,
    color_mode: Option<String>,
    palette: Option<String>,
    arrow_gap_threshold: Option<usize>,
    focus_branch_oid: Option<String>,
) -> Result<gitv_git_core::graph::GraphLayout, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let repo_for_blocking = Arc::clone(&repo);
    let repo_path_for_blocking = repo_path.clone();
    let options = parse_graph_options(
        hide_merges,
        orientation,
        color_mode,
        palette,
        arrow_gap_threshold,
    );

    let (layout, all_commits) = tauri::async_runtime::spawn_blocking(
        move || -> Result<(gitv_git_core::graph::GraphLayout, Vec<CommitInfo>), String> {
            let stashes = repo_for_blocking.stash_list().map_err(|e| e.to_string())?;
            let stash_parent_tips: Vec<Oid> = stashes.iter().map(|s| s.parent_oid).collect();

            let repo_info = repo_for_blocking.info().map_err(|e| e.to_string())?;
            let (commits, _working_changes) = load_commits_with_virtuals(
                &*repo_for_blocking,
                &repo_path_for_blocking,
                &stash_parent_tips,
                repo_info.head_commit.as_ref(),
            )?;

            let refs_map = build_refs_map(&commits);

            let calc = GraphCalculator::new(commits, refs_map, stashes, options);
            let mut layout = calc.calculate_layout();

            if let Some(ref oid_hex) = focus_branch_oid
                && let Ok(focus_oid) = Oid::from_hex(oid_hex)
            {
                let ancestors = calc.get_ancestor_oids(&focus_oid);
                GraphCalculator::apply_dimming(&mut layout, Some(focus_oid), Some(&ancestors));
            }

            let mut all_commits = calc.into_commits();

            // Sort commits by layout row to keep the stored order consistent
            // with the (possibly reversed) layout, so lazy-loaded batches
            // always correspond to sequential rows.
            let row_map: HashMap<Oid, usize> =
                layout.nodes.iter().map(|n| (n.oid, n.row)).collect();
            all_commits.sort_by_key(|c| row_map.get(&c.oid).copied().unwrap_or(usize::MAX));

            Ok((layout, all_commits))
        },
    )
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))??;

    state.store_commits(&repo_path, all_commits);
    Ok(layout)
}
