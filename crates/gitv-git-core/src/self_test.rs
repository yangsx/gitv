use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::error::GitError;
use crate::gix_repo::GixRepository;
use crate::graph::{GraphCalculator, GraphOptions, LayoutDiagnostics, TopologySummary};
use crate::models::Oid;
use crate::repository::Repository;

/// Full output of a self-test run.
#[derive(Serialize)]
pub struct SelfTestOutput {
    /// Canonical path of the repository.
    pub repo_path: String,
    /// Directory basename of the repository (e.g. "linux").
    pub repo_name: String,
    /// Wall-clock time from start to finish (open repo, load commits, compute, verify).
    pub timing_ms: f64,
    /// Time spent in compute (graph layout + verify + diagnose).
    pub compute_ms: f64,
    /// Number of nodes in the layout.
    pub node_count: usize,
    /// Number of edges in the layout.
    pub edge_count: usize,
    /// Total columns used.
    pub total_columns: usize,
    /// Number of pass-through errors found by verify().
    pub error_count: usize,
    /// Up to 1000 error messages.
    pub errors: Vec<String>,
    /// Layout quality diagnostics.
    pub diagnostics: LayoutDiagnostics,
    /// Commit topology summary.
    pub topology: TopologySummary,
    /// Comma-separated column shift histogram: "0:5123,1:1845,2:192"
    pub column_shift_histogram: String,
    /// Comma-separated row thread histogram: "1:234,2:567,3:89"
    pub row_thread_histogram: String,
    /// Number of nodes with hide_merges enabled.
    pub hide_merges_node_count: usize,
    /// Number of edges with hide_merges enabled.
    pub hide_merges_edge_count: usize,
    /// Number of pass-through errors with hide_merges enabled.
    pub hide_merges_error_count: usize,
    /// Up to 1000 error messages from the hide_merges layout.
    pub hide_merges_errors: Vec<String>,
}

/// Run a self-test on the repository at `path`.
///
/// Opens the repo, loads commits + stashes, computes the graph layout,
/// runs verification and diagnostics, and returns structured results.
///
/// `max_commits` limits how many commits are loaded. `None` = no limit.
pub fn run_self_test(path: &Path, max_commits: Option<usize>) -> Result<SelfTestOutput, GitError> {
    let overall = Instant::now();

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let repo_path = canonical.to_string_lossy().to_string();
    let repo_name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let repo = GixRepository::open(path)?;
    tracing::info!("self_test: open repo in {:?}", overall.elapsed());

    let stashes = repo.stash_list()?;
    tracing::info!(
        "self_test: {} stashes loaded in {:?}",
        stashes.len(),
        overall.elapsed()
    );

    let stash_parent_tips: Vec<Oid> = stashes.iter().map(|s| s.parent_oid).collect();

    let commits = repo.commits(max_commits, &stash_parent_tips)?;
    tracing::info!(
        "self_test: {} commits loaded in {:?}",
        commits.len(),
        overall.elapsed()
    );

    let compute_start = Instant::now();
    let calc = GraphCalculator::new(
        commits.clone(),
        HashMap::new(),
        stashes.clone(),
        GraphOptions::default(),
    );
    let layout = calc.calculate_layout();

    let errors = layout.verify();
    let error_count = errors.len();
    let diag = layout.diagnose();
    let topo = layout.topology_summary();
    let compute_ms = compute_start.elapsed().as_secs_f64() * 1000.0;

    let node_count = layout.nodes.len();
    let edge_count = layout.edges.len();
    let total_cols = layout.total_columns;

    tracing::info!(
        "self_test: compute in {:.1}ms \
         ({node_count} nodes, {edge_count} edges, {total_cols} cols, {error_count} errors)",
        compute_ms,
    );

    let max_errors_shown = 1000;
    let shown_errors: Vec<String> = errors.into_iter().take(max_errors_shown).collect();

    // Run hide_merges layout to catch edge routing bugs that only manifest
    // when merge commits are filtered out.
    let hide_calc = GraphCalculator::new(
        commits,
        HashMap::new(),
        stashes,
        GraphOptions {
            hide_merges: true,
            ..GraphOptions::default()
        },
    );
    let hide_layout = hide_calc.calculate_layout();
    let hide_errors = hide_layout.verify();
    let hide_error_count = hide_errors.len();
    let hide_node_count = hide_layout.nodes.len();
    let hide_edge_count = hide_layout.edges.len();

    tracing::info!(
        "self_test: hide_merges \
         ({hide_node_count} nodes, {hide_edge_count} edges, {hide_error_count} errors)",
    );

    let hide_shown_errors: Vec<String> = hide_errors.into_iter().take(max_errors_shown).collect();

    let col_shift_hist: String = diag
        .column_shift_histogram
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .map(|(d, c)| format!("{d}:{c}"))
        .collect::<Vec<_>>()
        .join(",");

    let row_thread_hist: String = diag
        .row_thread_histogram
        .iter()
        .enumerate()
        .filter(|(_, c)| **c > 0)
        .map(|(t, c)| format!("{t}:{c}"))
        .collect::<Vec<_>>()
        .join(",");

    Ok(SelfTestOutput {
        repo_path,
        repo_name,
        timing_ms: overall.elapsed().as_secs_f64() * 1000.0,
        compute_ms,
        node_count,
        edge_count,
        total_columns: total_cols,
        error_count,
        errors: shown_errors,
        diagnostics: diag,
        topology: topo,
        column_shift_histogram: col_shift_hist,
        row_thread_histogram: row_thread_hist,
        hide_merges_node_count: hide_node_count,
        hide_merges_edge_count: hide_edge_count,
        hide_merges_error_count: hide_error_count,
        hide_merges_errors: hide_shown_errors,
    })
}
