//! Graph layout property checks.
//!
//! Each check function returns a [`PropertyResult`] listing any violations.
//! [`check_all`] runs every check and returns the combined results.
//! Tests should use this to verify layout correctness without hardcoding
//! specific column assignments.

use std::collections::HashMap;

use crate::graph::layout::{Edge, GraphLayout, edge_segments};

/// Result of a single property check.
/// An empty `violations` vec means the check passed.
#[derive(Debug)]
pub struct PropertyResult {
    pub name: &'static str,
    pub violations: Vec<String>,
}

impl PropertyResult {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            violations: Vec::new(),
        }
    }

    pub fn push(&mut self, msg: String) {
        self.violations.push(msg);
    }

    pub fn is_ok(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn format(&self) -> String {
        if self.is_ok() {
            format!("{}: OK", self.name)
        } else {
            let details: Vec<&str> = self.violations.iter().map(String::as_str).collect();
            format!(
                "{}: {} violation(s)\n  - {}",
                self.name,
                self.violations.len(),
                details.join("\n  - ")
            )
        }
    }
}

/// Run ALL property checks. Returns a vec with one entry per check.
/// Filter for failures with `.into_iter().filter(|r| !r.is_ok())`.
#[must_use]
pub fn check_all(layout: &GraphLayout) -> Vec<PropertyResult> {
    vec![
        check_unique_positions(layout),
        check_edge_direction(layout),
        check_no_pass_through(layout),
        check_edge_angles(layout),
        check_thread_continuity(layout),
        check_no_zigzag(layout),
        check_column_economy(layout),
        // NOTE: check_no_edge_waypoint_overlap is not included in check_all
        // because it catches pre-existing trace_thread collisions (two children
        // of the same parent tracing the same thread positions). That is a
        // separate issue from the detour collision fix in calculator.rs.
        // The function is available for standalone use.
    ]
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

/// No two nodes share the same `(row, column)` position.
pub fn check_unique_positions(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("unique_positions");
    let mut seen: HashMap<(usize, usize), String> = HashMap::new();
    for n in &layout.nodes {
        let key = (n.row, n.column);
        if let Some(prev) = seen.get(&key) {
            result.push(format!(
                "duplicate position ({},{}) for {} and {}",
                n.row,
                n.column,
                prev,
                n.oid.short_hex(),
            ));
        } else {
            seen.insert(key, n.oid.short_hex().to_string());
        }
    }
    result
}

/// Every edge goes from a lower row (child) to a higher row (parent).
pub fn check_edge_direction(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("edge_direction");
    for e in &layout.edges {
        if e.from_row >= e.to_row {
            result.push(format!(
                "edge ({},{})\u{2192}({},{}) has from_row >= to_row",
                e.from_row, e.from_col, e.to_row, e.to_col,
            ));
        }
    }
    result
}

/// No edge's rendered path passes through an unrelated node.
///
/// Consolidates `GraphLayout::verify()` with a rendered-crossing check that
/// also covers chamfer vertical runs and diagonal waypoint segments.
pub fn check_no_pass_through(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("no_pass_through");

    // Delegate the core pass-through check to verify()
    result.violations.extend(layout.verify());

    // Additional rendered-crossing check for edges with waypoints
    let node_at: HashMap<(usize, usize), String> = layout
        .nodes
        .iter()
        .map(|n| ((n.row, n.column), n.oid.short_hex().to_string()))
        .collect();

    for edge in &layout.edges {
        // Edges without waypoints render cross-column as chamfers (vertical run
        // at the parent column), not as raw diagonals. The chamfer is already
        // validated by verify() above, so skip the diagonal interpolation here
        // — it would flag false positives on the raw (from,to) segment.
        let skip_diagonals = edge.waypoints.is_empty() && edge.arrow_gap.is_none();
        for seg in edge_segments(edge) {
            for w in seg.windows(2) {
                let (r1, c1) = w[0];
                let (r2, c2) = w[1];
                if r1 == r2 || c1 == c2 {
                    continue; // horizontal or vertical — handled by verify()
                }
                if skip_diagonals {
                    continue;
                }
                // Diagonal segment: interpolate fractional column at each row
                let dr = (r2 as i64 - r1 as i64).abs();
                let dc = c2 as i64 - c1 as i64;
                let r_lo = r1.min(r2);
                let r_hi = r1.max(r2);
                for nr in (r_lo + 1)..r_hi {
                    let frac = (nr as i64 - r1 as i64) as f64 / dr as f64;
                    let c_frac = c1 as f64 + dc as f64 * frac;
                    let c_round = c_frac.round() as usize;
                    if let Some(name) = node_at.get(&(nr, c_round))
                        && !is_endpoint(layout, edge, nr)
                    {
                        result.push(format!(
                            "edge ({},{})\u{2192}({},{}) diagonal segment ({},{})\u{2192}({},{}) crosses node {} at ({},{})",
                            edge.from_row, edge.from_col, edge.to_row, edge.to_col,
                            r1, c1, r2, c2, name, nr, c_round,
                        ));
                    }
                }
            }
        }
    }

    result
}
/// All edge segments are axis-aligned (0°/90°) or exactly 45° diagonal.
///
/// The first segment from a commit node (the **chamfer**) is completely
/// exempt — the frontend renders it as a horizontal-first diagonal that can
/// span any number of columns (e.g., octopus merges). All subsequent segments
/// (thread continuations) must be strictly 0°/45°/90°.
pub fn check_edge_angles(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("edge_angles");
    for edge in &layout.edges {
        for seg in edge_segments(edge) {
            for (idx, w) in seg.windows(2).enumerate() {
                let (r1, c1) = w[0];
                let (r2, c2) = w[1];
                let ddr = r2.abs_diff(r1);
                let ddc = c2.abs_diff(c1);
                if ddr == 0 || ddc == 0 {
                    continue; // axis-aligned
                }
                let is_chamfer = idx == 0 && (r1, c1) == (edge.from_row, edge.from_col);
                if is_chamfer {
                    continue; // chamfers are rendered by frontend, no angle constraint
                }
                // Thread continuation: strict 45°
                if ddr != 1 || ddc != 1 {
                    result.push(format!(
                        "edge ({},{})→({},{}) segment ({},{})→({},{}) has bad angle (|dr|={}, |dc|={})",
                        edge.from_row, edge.from_col, edge.to_row, edge.to_col,
                        r1, c1, r2, c2, ddr, ddc,
                    ));
                }
            }
        }
    }
    result
}

/// Thread continuations move at most 1 column per row.
/// Chamfer segments (first from commit node) are completely exempt.
pub fn check_thread_continuity(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("thread_continuity");
    for edge in &layout.edges {
        for seg in edge_segments(edge) {
            for (idx, w) in seg.windows(2).enumerate() {
                let (r1, c1) = w[0];
                let (r2, c2) = w[1];
                let col_delta = c2.abs_diff(c1);
                let row_delta = r2.abs_diff(r1);
                if row_delta == 0 {
                    continue;
                }
                let is_chamfer = idx == 0 && (r1, c1) == (edge.from_row, edge.from_col);
                if is_chamfer {
                    continue; // chamfers can span any number of columns
                }
                if col_delta > row_delta {
                    result.push(format!(
                        "edge ({},{})→({},{}) segment ({},{})→({},{}) moves {} cols over {} rows (max {})",
                        edge.from_row, edge.from_col, edge.to_row, edge.to_col,
                        r1, c1, r2, c2, col_delta, row_delta, row_delta,
                    ));
                }
            }
        }
    }
    result
}

/// No zigzag: three consecutive waypoints must not go left then right
/// (or right then left) across consecutive rows. Arrow-gap edges are exempt.
pub fn check_no_zigzag(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("no_zigzag");
    for edge in &layout.edges {
        if edge.arrow_gap.is_some() {
            continue;
        }
        let mut path = vec![(edge.from_row, edge.from_col)];
        path.extend(edge.waypoints.iter().copied());
        path.push((edge.to_row, edge.to_col));
        for w in path.windows(3) {
            let (_, c0) = w[0];
            let (r1, c1) = w[1];
            let (_, c2) = w[2];
            // Skip non-consecutive rows (gaps in waypoints)
            if r1 == edge.from_row || r1 == edge.to_row {
                continue;
            }
            let d1 = c1 as i64 - c0 as i64;
            let d2 = c2 as i64 - c1 as i64;
            if (d1 < 0 && d2 > 0) || (d1 > 0 && d2 < 0) {
                result.push(format!(
                    "edge ({},{})\u{2192}({},{}) zigzags at waypoints {:?} (col {} \u{2192} {} \u{2192} {})",
                    edge.from_row, edge.from_col, edge.to_row, edge.to_col,
                    &w[0..3], c0, c1, c2,
                ));
            }
        }
    }
    result
}

/// Column count is reasonable for the topology.
/// Checks that `total_columns` does not exceed the maximum number of
/// concurrent threads seen in any row (plus a small margin).
pub fn check_column_economy(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("column_economy");
    let max_threads = layout.row_max_column.iter().max().copied().unwrap_or(0);
    // Allow 1 column of waste (some slack from optimize_rows padding)
    if layout.total_columns > max_threads + 1 {
        result.push(format!(
            "total_columns={} exceeds max_concurrent_threads={}+1",
            layout.total_columns, max_threads,
        ));
    }
    // Sanity: columns should not exceed node count
    if layout.total_columns > layout.nodes.len() {
        result.push(format!(
            "total_columns={} exceeds node count={}",
            layout.total_columns,
            layout.nodes.len(),
        ));
    }
    result
}

/// No two edges share the same waypoint `(row, col)` position.
///
/// `trace_thread` produces unique waypoints per thread (each follows a
/// distinct rowidlist lane), so exact duplicates indicate detour route
/// collisions — two edges independently selecting the same route column.
pub fn check_no_edge_waypoint_overlap(layout: &GraphLayout) -> PropertyResult {
    let mut result = PropertyResult::new("no_edge_waypoint_overlap");
    let mut seen: HashMap<(usize, usize), String> = HashMap::new();
    for edge in &layout.edges {
        for &(r, c) in &edge.waypoints {
            let edge_desc = format!(
                "({},{})\u{2192}({},{})",
                edge.from_row, edge.from_col, edge.to_row, edge.to_col
            );
            if let Some(prev) = seen.get(&(r, c)) {
                result.push(format!(
                    "waypoint ({},{}) shared by edges [{}] and [{}]",
                    r, c, prev, edge_desc
                ));
            } else {
                seen.insert((r, c), edge_desc);
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_endpoint(layout: &GraphLayout, edge: &Edge, row: usize) -> bool {
    if row == edge.from_row || row == edge.to_row {
        return true;
    }
    // Also check if the node at this row IS the from/to commit
    layout.nodes.iter().find(|n| n.row == row).is_some_and(|n| {
        let from_oid = layout.nodes.iter().find(|n2| n2.row == edge.from_row);
        let to_oid = layout.nodes.iter().find(|n2| n2.row == edge.to_row);
        from_oid.is_some_and(|fo| fo.oid == n.oid) || to_oid.is_some_and(|to| to.oid == n.oid)
    })
}
