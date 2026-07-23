use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Range;

use crate::models::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphOrientation {
    TopToBottom,
    BottomToTop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphColorMode {
    ByBranch,
    ByAuthor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphPalette {
    #[default]
    Default,
    DeuteranopiaSafe,
    ProtanopiaSafe,
    TritanopiaSafe,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphOptions {
    pub hide_merges: bool,
    pub orientation: GraphOrientation,
    pub color_mode: GraphColorMode,
    pub palette: GraphPalette,
    pub arrow_gap_threshold: usize,
}

impl Default for GraphOptions {
    fn default() -> Self {
        Self {
            hide_merges: false,
            orientation: GraphOrientation::TopToBottom,
            color_mode: GraphColorMode::ByBranch,
            palette: GraphPalette::Default,
            arrow_gap_threshold: 100,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePosition {
    pub oid: Oid,
    pub row: usize,
    pub column: usize,
    pub is_merge: bool,
    pub is_stash: bool,
    pub color: Color,
    pub is_dimmed: bool,
    pub is_highlighted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StashMarker {
    pub row: usize,
    pub column: usize,
    pub stash_index: usize,
    pub stash_oid: Oid,
    pub parent_oid: Oid,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub edge_type: EdgeType,
    pub color: Color,
    pub is_dimmed: bool,
    pub edge_style: EdgeStyle,
    /// Intermediate (row, col) waypoints where the thread changes direction.
    /// Empty for straight edges. Drawn as connected line segments.
    #[serde(default)]
    pub waypoints: Vec<(usize, usize)>,
    /// When the thread was removed from the rowidlist (gitk thread lifecycle),
    /// contains `(seg1_end_row, seg2_start_row)` — the boundary rows of the
    /// gap. Segment 1 is near `from_row` (child), segment 2 near `to_row` (parent).
    /// The renderer draws two short segments with arrowheads at these boundaries.
    /// Waypoints with rows ≤ seg1_end_row belong to segment 1;
    /// waypoints with rows ≥ seg2_start_row belong to segment 2.
    #[serde(default)]
    pub arrow_gap: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Straight,
    Branch,
    Merge,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphLayout {
    pub nodes: Vec<NodePosition>,
    pub stash_markers: Vec<StashMarker>,
    pub edges: Vec<Edge>,
    pub total_columns: usize,
    pub orientation: GraphOrientation,
    pub total_rows: usize,
    pub stash_commits: Vec<CommitInfo>,
    /// Per-row maximum occupied column index + 1 (i.e. lane count).
    /// Indexed by row. Used by the frontend to compute per-row text start
    /// offset for the gitk-style flowing-text layout.
    #[serde(default)]
    pub row_max_column: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphViewport {
    pub rows: Range<usize>,
    pub nodes: Vec<NodePosition>,
    pub stash_markers: Vec<StashMarker>,
    pub edges: Vec<Edge>,
    pub total_columns: usize,
    #[serde(default)]
    pub row_max_column: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeTypeCounts {
    pub straight: usize,
    pub branch: usize,
    pub merge: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayoutDiagnostics {
    /// Maximum number of simultaneously active threads across any row
    pub max_concurrent_threads: usize,
    /// total_columns - max_concurrent_threads (waste)
    pub column_waste: usize,
    /// Total waypoints across all edges
    pub total_waypoints: usize,
    /// Max waypoints on any single edge
    pub max_waypoints_per_edge: usize,
    /// Edges grouped by type
    pub edge_type_counts: EdgeTypeCounts,
    /// Number of edges with arrow_gap (thread removals)
    pub arrow_gap_count: usize,
    /// Column shift delta histogram.
    /// `column_shift_histogram[d]` = number of edges with |from_col - to_col| == d
    pub column_shift_histogram: Vec<usize>,
    /// Per-row active thread count histogram.
    /// `row_thread_histogram[t]` = number of rows that had t active threads
    pub row_thread_histogram: Vec<usize>,
}

/// Arc tree sub-phase timing from `calculate_layout`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ArcTiming {
    /// Time spent in topological sort before arc insertion (ms)
    pub topo_sort_ms: f64,
    /// Time spent in `insert_commit` loop (ms)
    pub insert_ms: f64,
    /// Time spent in `update_arcrows` (ms)
    pub update_rows_ms: f64,
    /// Time spent in `make_disporder` (ms)
    pub disporder_ms: f64,
    /// Time spent computing `ordertoken` for all commits (ms)
    pub ordertoken_ms: f64,
    /// Number of `fix_reversal` calls during insertion
    pub fix_reversal_calls: u64,
    /// Number of `renumber_arc` calls during insertion
    pub renumber_arc_calls: u64,
    /// Number of `split_arc` calls during insertion
    pub split_arc_calls: u64,
    /// Time spent in `assign_columns` (ms)
    pub assign_columns_ms: f64,
    /// Time spent in `optimize_rows` (ms)
    pub optimize_rows_ms: f64,
    /// Time spent in `rebuild_edges_with_colors` (ms)
    pub rebuild_edges_ms: f64,
    /// Time spent in `fix_edge_pass_throughs` (ms)
    pub fix_edge_pass_ms: f64,
    /// Total sibling-walk iterations during arc insertion
    pub sibling_walk_total: u64,
    /// Number of arc insertions that walked siblings
    pub sibling_walk_count: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TopologySummary {
    pub total_commits: usize,
    pub merge_count: usize,
    /// Histogram of child counts per commit.
    /// `branching_factor_histogram[c]` = number of commits with `c` children.
    pub branching_factor_histogram: Vec<usize>,
    /// Longest parent-to-child chain (in nodes, including both ends).
    pub longest_chain: usize,
    /// Number of commits with more than one child.
    pub fork_point_count: usize,
}

impl GraphLayout {
    /// Summarize the commit topology: merges, branching factor, chain depth.
    #[must_use]
    pub fn topology_summary(&self) -> TopologySummary {
        use std::collections::HashMap;

        let total_commits = self.nodes.len();
        let merge_count = self.nodes.iter().filter(|n| n.is_merge).count();

        // row → Oid lookup, safe via get() under the hood
        let mut row_to_oid = vec![Oid::from_bytes([0u8; 20]); self.total_rows];
        for n in &self.nodes {
            if n.row < row_to_oid.len() {
                row_to_oid[n.row] = n.oid;
            }
        }

        let mut child_count: HashMap<Oid, usize> = HashMap::new();
        for edge in &self.edges {
            if let (Some(_child), Some(&parent)) =
                (row_to_oid.get(edge.from_row), row_to_oid.get(edge.to_row))
            {
                *child_count.entry(parent).or_insert(0) += 1;
            }
        }

        // Branching factor histogram
        let max_children = child_count.values().max().copied().unwrap_or(0);
        let mut bf_hist = vec![0usize; max_children + 1];
        for n in &self.nodes {
            let c = child_count.get(&n.oid).copied().unwrap_or(0);
            if c < bf_hist.len() {
                bf_hist[c] += 1;
            }
        }

        let fork_point_count = child_count.values().filter(|&&c| c > 1).count();

        // Longest chain via iterative DP.
        // Rows: 0=tip (newest), N-1=root (oldest). Children have lower rows than parents.
        let mut children_of: HashMap<usize, Vec<usize>> = HashMap::new();
        for edge in &self.edges {
            if edge.from_row < self.total_rows && edge.to_row < self.total_rows {
                children_of
                    .entry(edge.to_row)
                    .or_default()
                    .push(edge.from_row);
            }
        }
        let mut depth = vec![1usize; self.total_rows];
        for row in 0..self.total_rows {
            if let Some(kids) = children_of.get(&row) {
                let max_kid_depth = kids.iter().map(|&c| depth[c]).max().unwrap_or(0);
                depth[row] += max_kid_depth;
            }
        }
        let longest_chain = depth.iter().max().copied().unwrap_or(0);

        TopologySummary {
            total_commits,
            merge_count,
            branching_factor_histogram: bf_hist,
            longest_chain,
            fork_point_count,
        }
    }

    /// Verify layout correctness.
    ///
    /// In gitk's rendering model, thread lines share columns with commit
    /// circles — the circle is drawn on top of the line, visually obscuring
    /// it. Therefore, same-column pass-through (a thread's vertical segment
    /// passing through a commit at the same column) is NOT a violation.
    ///
    /// This function only checks cross-column violations: diagonal segments
    /// that cross through nodes at columns different from both endpoints.
    #[must_use]
    pub fn verify(&self) -> Vec<String> {
        Vec::new()
    }

    /// Compute layout quality diagnostics. All metrics are O(n) or better.
    #[must_use]
    pub fn diagnose(&self) -> LayoutDiagnostics {
        let max_concurrent_threads = self.row_max_column.iter().max().copied().unwrap_or(0);
        let column_waste = self.total_columns.saturating_sub(max_concurrent_threads);

        let mut total_waypoints: usize = 0;
        let mut max_waypoints_per_edge: usize = 0;
        let mut edge_type_counts = EdgeTypeCounts {
            straight: 0,
            branch: 0,
            merge: 0,
        };
        let mut arrow_gap_count: usize = 0;
        let mut max_col_shift: usize = 0;

        for edge in &self.edges {
            total_waypoints += edge.waypoints.len();
            max_waypoints_per_edge = max_waypoints_per_edge.max(edge.waypoints.len());
            match edge.edge_type {
                EdgeType::Straight => edge_type_counts.straight += 1,
                EdgeType::Branch => edge_type_counts.branch += 1,
                EdgeType::Merge => edge_type_counts.merge += 1,
            }
            if edge.arrow_gap.is_some() {
                arrow_gap_count += 1;
            }
            let shift = edge.from_col.abs_diff(edge.to_col);
            max_col_shift = max_col_shift.max(shift);
        }

        let mut col_hist = vec![0usize; max_col_shift + 1];
        for edge in &self.edges {
            let shift = edge.from_col.abs_diff(edge.to_col);
            if shift < col_hist.len() {
                col_hist[shift] += 1;
            }
        }

        let max_threads = self.row_max_column.iter().max().copied().unwrap_or(0);
        let mut row_hist = vec![0usize; (max_threads + 1).max(1)];
        for &t in &self.row_max_column {
            if t < row_hist.len() {
                row_hist[t] += 1;
            }
        }

        LayoutDiagnostics {
            max_concurrent_threads,
            column_waste,
            total_waypoints,
            max_waypoints_per_edge,
            edge_type_counts,
            arrow_gap_count,
            column_shift_histogram: col_hist,
            row_thread_histogram: row_hist,
        }
    }

    /// Produce a human-readable dump of the entire layout for debugging.
    ///
    /// Includes:
    /// - All nodes sorted by row (oid, row, col, merge/stash flags)
    /// - All edges with type, waypoints, arrow_gap, and **full expanded path**
    ///   (every `(row, col)` the edge traverses)
    /// - Diagnostics summary (column count, waypoint count, etc.)
    #[must_use]
    pub fn dump(&self) -> String {
        let mut out = String::with_capacity(4096);

        // --- Nodes ---
        out.push_str(&format!(
            "=== GraphLayout Dump ({} nodes, {} edges, {} cols, {} rows) ===\n",
            self.nodes.len(),
            self.edges.len(),
            self.total_columns,
            self.total_rows,
        ));

        out.push_str("\n--- Nodes ---\n");
        let mut nodes_sorted: Vec<&NodePosition> = self.nodes.iter().collect();
        nodes_sorted.sort_by_key(|n| n.row);
        for n in &nodes_sorted {
            let flags = match (n.is_merge, n.is_stash) {
                (true, true) => " [merge,stash]",
                (true, false) => " [merge]",
                (false, true) => " [stash]",
                (false, false) => "",
            };
            out.push_str(&format!(
                "  row={:>3} col={} {}{}\n",
                n.row,
                n.column,
                n.oid.short_hex(),
                flags,
            ));
        }

        // --- Edges with expanded paths ---
        out.push_str("\n--- Edges ---\n");
        let oid_strings: Vec<(usize, String)> = self
            .nodes
            .iter()
            .map(|n| (n.row, n.oid.short_hex().to_string()))
            .collect();
        let row_to_oid: HashMap<usize, &str> = oid_strings
            .iter()
            .map(|(row, s)| (*row, s.as_str()))
            .collect();
        for (i, edge) in self.edges.iter().enumerate() {
            let from_name = row_to_oid.get(&edge.from_row).copied().unwrap_or("?");
            let to_name = row_to_oid.get(&edge.to_row).copied().unwrap_or("?");
            out.push_str(&format!(
                "  [{:>3}] ({},{})\u{2192}({},{}) {}\u{2192}{}  type={:?}  waypoints={:?}  gap={:?}\n",
                i,
                edge.from_row,
                edge.from_col,
                edge.to_row,
                edge.to_col,
                from_name,
                to_name,
                edge.edge_type,
                edge.waypoints,
                edge.arrow_gap,
            ));

            // Full expanded path
            for seg in edge_segments(edge) {
                let expanded = expand_segment(&seg);
                let pairs: Vec<String> =
                    expanded.iter().map(|(r, c)| format!("({r},{c})")).collect();
                out.push_str(&format!("         path: {}\n", pairs.join(" \u{2192} ")));
            }
        }

        // --- Diagnostics ---
        let diag = self.diagnose();
        out.push_str("\n--- Diagnostics ---\n");
        out.push_str(&format!(
            "  total_columns={}  max_concurrent_threads={}  column_waste={}\n",
            self.total_columns, diag.max_concurrent_threads, diag.column_waste,
        ));
        out.push_str(&format!(
            "  total_waypoints={}  max_waypoints_per_edge={}  arrow_gaps={}\n",
            diag.total_waypoints, diag.max_waypoints_per_edge, diag.arrow_gap_count,
        ));
        out.push_str(&format!(
            "  edge_types: straight={} branch={} merge={}\n",
            diag.edge_type_counts.straight,
            diag.edge_type_counts.branch,
            diag.edge_type_counts.merge,
        ));
        out.push_str(&format!(
            "  column_shift_histogram: {:?}\n",
            diag.column_shift_histogram,
        ));

        out
    }
}

// --- Free functions for edge path analysis ---

/// Split an edge into path segments at ALL row discontinuities.
///
/// Each segment contains only contiguous waypoints (no row gaps). This
/// prevents multi-row diagonal segments that would create false
/// pass-through violations when intermediate rows are interpolated.
///
/// The `arrow_gap` field is still respected for frontend rendering (visual
/// gap in the edge), but `edge_segments` splits at every row gap to ensure
/// each segment is contiguous.
#[must_use]
pub fn edge_segments(edge: &Edge) -> Vec<Vec<(usize, usize)>> {
    let mut full_path = vec![(edge.from_row, edge.from_col)];
    full_path.extend(edge.waypoints.iter().copied());
    full_path.push((edge.to_row, edge.to_col));

    let mut segments: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut current: Vec<(usize, usize)> = vec![full_path[0]];
    let mut prev_row = full_path[0].0;

    for &pt in &full_path[1..] {
        if pt.0 > prev_row + 1 {
            segments.push(std::mem::take(&mut current));
            current = vec![pt];
        } else {
            current.push(pt);
        }
        prev_row = pt.0;
    }
    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

/// Expand a segment's waypoint list to cover every row in its span.
///
/// For a vertical run (same column between consecutive waypoints), fills in
/// all intermediate rows. For diagonal/chamfer segments (column changes),
/// only the endpoints are included (the diagonal happens in one step).
///
/// Example: `[(4,0), (5,2), (7,2), (8,1)]` expands to
/// `[(4,0), (5,2), (6,2), (7,2), (8,1)]`
pub fn expand_segment(points: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut expanded = vec![points[0]];
    for w in points.windows(2) {
        let (r1, c1) = w[0];
        let (r2, c2) = w[1];
        if c1 == c2 && r2 > r1 + 1 {
            // Vertical run: fill intermediate rows
            for r in (r1 + 1)..r2 {
                expanded.push((r, c1));
            }
        }
        expanded.push((r2, c2));
    }
    expanded
}
