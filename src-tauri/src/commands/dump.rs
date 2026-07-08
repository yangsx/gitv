use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::graph::{
    GraphCalculator, GraphLayout, GraphOptions, LayoutDiagnostics, check_all, edge_segments,
    expand_segment,
};
use gitv_git_core::models::{CommitInfo, Oid, Ref};
use gitv_git_core::repository::Repository;
use serde::Serialize;

use super::graph::{build_refs_map, make_virtual_commits};

fn serialize_oid<S: serde::Serializer>(oid: &Oid, s: S) -> Result<S::Ok, S::Error> {
    use gitv_git_core::models::{STAGED_OID, UNSTAGED_OID};
    if *oid == STAGED_OID {
        return s.serialize_str("__staged__");
    }
    if *oid == UNSTAGED_OID {
        return s.serialize_str("__unstaged__");
    }
    s.serialize_str(&oid.to_hex())
}

fn serialize_oid_vec<S: serde::Serializer>(oids: &[Oid], s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeSeq;
    let mut seq = s.serialize_seq(Some(oids.len()))?;
    for oid in oids {
        let oid_str = if *oid == gitv_git_core::models::STAGED_OID {
            "__staged__".to_string()
        } else if *oid == gitv_git_core::models::UNSTAGED_OID {
            "__unstaged__".to_string()
        } else {
            oid.to_hex()
        };
        seq.serialize_element(&oid_str)?;
    }
    seq.end()
}

fn serialize_oid_option<S: serde::Serializer>(oid: &Option<Oid>, s: S) -> Result<S::Ok, S::Error> {
    match oid {
        Some(o) => serialize_oid(o, s),
        None => s.serialize_none(),
    }
}

/// A single node in the dump — merges NodePosition with CommitInfo.
#[derive(Serialize)]
pub struct DumpNode {
    pub row: usize,
    pub column: usize,
    #[serde(serialize_with = "serialize_oid")]
    pub oid: Oid,
    pub is_merge: bool,
    pub is_stash: bool,
    pub is_virtual: bool,
    pub is_dimmed: bool,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    pub commit_time: String,
    #[serde(serialize_with = "serialize_oid_vec")]
    pub parent_oids: Vec<Oid>,
    pub refs: Vec<Ref>,
}

/// Result of a property check.
#[derive(Serialize)]
pub struct DumpPropertyCheck {
    pub name: String,
    pub violation_count: usize,
    pub sample: Vec<String>,
}

/// An edge with its fully expanded path — every (row, col) cell traversed.
///
/// `expanded_path` has one or two segments (two when there's an arrow_gap).
/// Each segment is a list of `(row, col)` pairs covering every row in its span.
#[derive(Serialize)]
pub struct DumpEdge {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub edge_type: gitv_git_core::graph::EdgeType,
    pub edge_style: gitv_git_core::graph::EdgeStyle,
    pub is_dimmed: bool,
    pub waypoints: Vec<(usize, usize)>,
    pub arrow_gap: Option<(usize, usize)>,
    pub expanded_path: Vec<Vec<(usize, usize)>>,
}

impl DumpEdge {
    fn from_edge(edge: &gitv_git_core::graph::Edge) -> Self {
        let segments = edge_segments(edge);
        let expanded_path: Vec<Vec<(usize, usize)>> =
            segments.iter().map(|seg| expand_segment(seg)).collect();
        DumpEdge {
            from_row: edge.from_row,
            from_col: edge.from_col,
            to_row: edge.to_row,
            to_col: edge.to_col,
            edge_type: edge.edge_type,
            edge_style: edge.edge_style,
            is_dimmed: edge.is_dimmed,
            waypoints: edge.waypoints.clone(),
            arrow_gap: edge.arrow_gap,
            expanded_path,
        }
    }
}

/// Complete graph layout dump for external analysis.
#[derive(Serialize)]
pub struct GraphDump {
    pub repo_path: String,
    #[serde(serialize_with = "serialize_oid_option")]
    pub head_oid: Option<Oid>,
    pub head_branch: Option<String>,
    pub options: GraphOptions,
    pub total_commits: usize,
    pub total_columns: usize,
    pub total_rows: usize,
    pub orientation: String,
    pub timing_ms: f64,
    pub nodes: Vec<DumpNode>,
    pub edges: Vec<DumpEdge>,
    pub row_max_column: Vec<usize>,
    pub stash_markers: Vec<gitv_git_core::graph::StashMarker>,
    pub diagnostics: LayoutDiagnostics,
    pub property_checks: Vec<DumpPropertyCheck>,
}

/// Compute the graph layout for the repository at `path` and return a
/// full dump suitable for JSON serialization.
///
/// Virtual commits (staged/unstaged) are included so the dump reflects
/// the real layout seen by the frontend.
pub fn dump_graph(
    path: &Path,
    max_commits: Option<usize>,
    options: GraphOptions,
) -> Result<GraphDump, String> {
    let start = Instant::now();

    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let repo_path = canonical.to_string_lossy().to_string();

    let repo = GixRepository::open(path).map_err(|e| e.to_string())?;
    let repo_info = repo.info().map_err(|e| e.to_string())?;

    let stashes = repo.stash_list().map_err(|e| e.to_string())?;
    let stash_parent_tips: Vec<Oid> = stashes.iter().map(|s| s.parent_oid).collect();

    let commits = repo
        .commits(max_commits, &stash_parent_tips)
        .map_err(|e| e.to_string())?;

    let working_changes = repo.working_changes_diff().ok();
    let virtuals = make_virtual_commits(repo_info.head_commit.as_ref(), working_changes.as_ref());

    let mut all_commits = virtuals;
    all_commits.extend(commits);

    let refs_map = build_refs_map(&all_commits);

    let calc = GraphCalculator::new(all_commits, refs_map, stashes, options.clone());
    let layout = calc.calculate_layout();
    let all_commits_back = calc.into_commits();

    let diagnostics = layout.diagnose();

    let property_checks: Vec<DumpPropertyCheck> = check_all(&layout)
        .into_iter()
        .map(|r| DumpPropertyCheck {
            name: r.name.to_string(),
            violation_count: r.violations.len(),
            sample: r.violations.into_iter().take(50).collect(),
        })
        .collect();

    let dump = build_dump(
        &repo_path,
        &repo_info.head_commit,
        &repo_info.head_branch,
        options,
        &layout,
        &all_commits_back,
        &diagnostics,
        property_checks,
        start.elapsed().as_secs_f64() * 1000.0,
    );

    Ok(dump)
}

#[allow(clippy::too_many_arguments)]
fn build_dump(
    repo_path: &str,
    head_oid: &Option<Oid>,
    head_branch: &Option<String>,
    options: GraphOptions,
    layout: &GraphLayout,
    commits: &[CommitInfo],
    diagnostics: &LayoutDiagnostics,
    property_checks: Vec<DumpPropertyCheck>,
    timing_ms: f64,
) -> GraphDump {
    let commit_map: HashMap<Oid, &CommitInfo> = commits.iter().map(|c| (c.oid, c)).collect();

    let nodes = layout
        .nodes
        .iter()
        .map(|n| {
            let ci = commit_map.get(&n.oid);
            DumpNode {
                row: n.row,
                column: n.column,
                oid: n.oid,
                is_merge: n.is_merge,
                is_stash: n.is_stash,
                is_virtual: n.oid.is_virtual(),
                is_dimmed: n.is_dimmed,
                message: ci.map(|c| c.summary().to_string()).unwrap_or_default(),
                author_name: ci.map(|c| c.author.name.clone()).unwrap_or_default(),
                author_email: ci.map(|c| c.author.email.clone()).unwrap_or_default(),
                commit_time: ci.map(|c| c.commit_time.to_rfc3339()).unwrap_or_default(),
                parent_oids: ci.map(|c| c.parent_oids.clone()).unwrap_or_default(),
                refs: ci.map(|c| c.refs.clone()).unwrap_or_default(),
            }
        })
        .collect();

    let edges: Vec<DumpEdge> = layout.edges.iter().map(DumpEdge::from_edge).collect();

    GraphDump {
        repo_path: repo_path.to_string(),
        head_oid: *head_oid,
        head_branch: head_branch.clone(),
        options,
        total_commits: commits.len(),
        total_columns: layout.total_columns,
        total_rows: layout.total_rows,
        orientation: format!("{:?}", layout.orientation),
        timing_ms,
        nodes,
        edges,
        row_max_column: layout.row_max_column.clone(),
        stash_markers: layout.stash_markers.clone(),
        diagnostics: diagnostics.clone(),
        property_checks,
    }
}

/// Entry point for the `--dump-graph` / `--dump-graph-json` CLI flags.
pub fn run_dump_graph(
    path: &Path,
    json: bool,
    max_commits: Option<usize>,
    options: GraphOptions,
) -> ! {
    match dump_graph(path, max_commits, options) {
        Ok(dump) => {
            if json {
                match serde_json::to_string_pretty(&dump) {
                    Ok(s) => println!("{s}"),
                    Err(e) => {
                        eprintln!("dump-graph serialization failed: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                // Text format: use the layout's built-in dump, then print
                // commit messages alongside nodes for human readability.
                print_dump_text(&dump);
            }
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("dump-graph failed: {e}");
            std::process::exit(1);
        }
    }
}

fn print_dump_text(dump: &GraphDump) {
    let GraphDump {
        repo_path,
        head_oid,
        head_branch,
        options,
        total_commits,
        total_columns,
        total_rows,
        timing_ms,
        nodes,
        edges,
        diagnostics,
        property_checks,
        ..
    } = dump;

    eprintln!(
        "dump-graph: {repo_path} — {total_commits} commits, {total_columns} cols, {total_rows} rows, {:.1}ms",
        timing_ms
    );
    eprintln!(
        "  options: hide_merges={}, orientation={:?}",
        options.hide_merges, options.orientation
    );
    if let Some(h) = head_oid {
        eprintln!("  head: {h} ({})", head_branch.as_deref().unwrap_or("?"));
    }

    eprintln!("\n--- Nodes (top 50) ---");
    for n in nodes.iter().take(50) {
        let oid_display: String = if n.is_virtual {
            if n.oid == gitv_git_core::models::STAGED_OID {
                "__staged__".to_string()
            } else {
                "__unstaged__".to_string()
            }
        } else {
            n.oid.short_hex()
        };
        let vflag = if n.is_virtual { " [virtual]" } else { "" };
        let mflag = if n.is_merge { " [merge]" } else { "" };
        let msg = if n.message.is_empty() {
            ""
        } else {
            // Truncate long messages
            &n.message[..n.message.len().min(60)]
        };
        eprintln!(
            "  row={:>3} col={} {}{}{} {}",
            n.row, n.column, oid_display, vflag, mflag, msg
        );
    }
    if nodes.len() > 50 {
        eprintln!("  ... ({} more)", nodes.len() - 50);
    }

    eprintln!("\n--- Edges (top 30) ---");
    for e in edges.iter().take(30) {
        eprintln!(
            "  ({},{})\u{2192}({},{}) type={:?} style={:?} wps={:?} gap={:?}",
            e.from_row,
            e.from_col,
            e.to_row,
            e.to_col,
            e.edge_type,
            e.edge_style,
            e.waypoints,
            e.arrow_gap
        );
        // Show expanded path (compact)
        for (si, seg) in e.expanded_path.iter().enumerate() {
            let label = if e.expanded_path.len() > 1 {
                format!("  seg{}:", si)
            } else {
                "  path:".to_string()
            };
            let cells: Vec<String> = seg.iter().map(|(r, c)| format!("({r},{c})")).collect();
            eprintln!("{label} {}", cells.join(" "));
        }
    }
    if edges.len() > 30 {
        eprintln!("  ... ({} more)", edges.len() - 30);
    }

    eprintln!("\n--- Diagnostics ---");
    eprintln!(
        "  column_waste={}, waypoints={}, max_waypoints_per_edge={}, arrow_gaps={}, max_threads={}",
        diagnostics.column_waste,
        diagnostics.total_waypoints,
        diagnostics.max_waypoints_per_edge,
        diagnostics.arrow_gap_count,
        diagnostics.max_concurrent_threads,
    );

    eprintln!("\n--- Property checks ---");
    for pc in property_checks {
        let status = if pc.violation_count == 0 {
            "OK"
        } else {
            "FAIL"
        };
        eprintln!(
            "  {status:<4} {} ({} violations)",
            pc.name, pc.violation_count
        );
    }
}
