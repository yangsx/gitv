use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use serde::Serialize;

use crate::error::GitError;
use crate::gix_repo::GixRepository;
use crate::graph::{
    GraphCalculator, GraphLayout, GraphOptions, LayoutDiagnostics, TopologySummary, check_all,
};
use crate::models::Oid;
use crate::repository::Repository;

/// Result of a single property check (e.g. "no_pass_through", "edge_angles").
#[derive(Serialize)]
pub struct PropertyCheckResult {
    /// Check name (e.g. "unique_positions", "no_pass_through").
    pub name: String,
    /// Total number of violations found.
    pub violation_count: usize,
    /// Up to 50 sample violation messages.
    pub sample: Vec<String>,
}

fn run_property_checks(layout: &GraphLayout) -> Vec<PropertyCheckResult> {
    check_all(layout)
        .into_iter()
        .map(|r| PropertyCheckResult {
            name: r.name.to_string(),
            violation_count: r.violations.len(),
            sample: r.violations.into_iter().take(50).collect(),
        })
        .collect()
}

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
    /// Property check results for the main layout.
    pub property_checks: Vec<PropertyCheckResult>,
    /// Property check results for the hide_merges layout.
    pub hide_merges_property_checks: Vec<PropertyCheckResult>,
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

    let diag = layout.diagnose();
    let topo = layout.topology_summary();
    let compute_ms = compute_start.elapsed().as_secs_f64() * 1000.0;

    let node_count = layout.nodes.len();
    let edge_count = layout.edges.len();
    let total_cols = layout.total_columns;

    tracing::info!(
        "self_test: compute in {:.1}ms \
         ({node_count} nodes, {edge_count} edges, {total_cols} cols)",
        compute_ms,
    );

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
    let hide_node_count = hide_layout.nodes.len();
    let hide_edge_count = hide_layout.edges.len();

    tracing::info!(
        "self_test: hide_merges \
         ({hide_node_count} nodes, {hide_edge_count} edges)",
    );

    // Run property checks on both layouts (includes no_pass_through via verify()).
    let property_checks = run_property_checks(&layout);
    let hide_merges_property_checks = run_property_checks(&hide_layout);

    let total_prop_violations: usize = property_checks.iter().map(|c| c.violation_count).sum();
    let total_hide_prop_violations: usize = hide_merges_property_checks
        .iter()
        .map(|c| c.violation_count)
        .sum();
    tracing::info!(
        "self_test: property checks — {total_prop_violations} violations (hide_merges: {total_hide_prop_violations})",
    );

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
        diagnostics: diag,
        topology: topo,
        column_shift_histogram: col_shift_hist,
        row_thread_histogram: row_thread_hist,
        hide_merges_node_count: hide_node_count,
        hide_merges_edge_count: hide_edge_count,
        property_checks,
        hide_merges_property_checks,
    })
}
