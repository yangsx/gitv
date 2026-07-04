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
}

/// Run a self-test on the repository at `path`.
///
/// Opens the repo, loads up to 10K commits + stashes, computes the graph layout,
/// runs verification and diagnostics, and returns structured results.
pub fn run_self_test(path: &Path) -> Result<SelfTestOutput, GitError> {
    let overall = Instant::now();

    let repo = GixRepository::open(path)?;
    tracing::info!("self_test: open repo in {:?}", overall.elapsed());

    let stashes = repo.stash_list()?;
    tracing::info!(
        "self_test: {} stashes loaded in {:?}",
        stashes.len(),
        overall.elapsed()
    );

    let stash_parent_tips: Vec<Oid> = stashes.iter().map(|s| s.parent_oid).collect();

    const MAX_COMMITS: usize = 10_000;
    let commits = repo.commits(Some(MAX_COMMITS), &stash_parent_tips)?;
    tracing::info!(
        "self_test: {} commits loaded in {:?}",
        commits.len(),
        overall.elapsed()
    );

    let compute_start = Instant::now();
    let calc = GraphCalculator::new(commits, HashMap::new(), stashes, GraphOptions::default());
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
    })
}
