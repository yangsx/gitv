use std::collections::{HashMap, HashSet};

use crate::graph::layout::*;
use crate::models::*;

const BRANCH_PALETTE: &[Color] = &[
    Color {
        r: 79,
        g: 148,
        b: 205,
        a: 255,
    },
    Color {
        r: 230,
        g: 126,
        b: 34,
        a: 255,
    },
    Color {
        r: 46,
        g: 204,
        b: 113,
        a: 255,
    },
    Color {
        r: 231,
        g: 76,
        b: 60,
        a: 255,
    },
    Color {
        r: 155,
        g: 89,
        b: 182,
        a: 255,
    },
    Color {
        r: 241,
        g: 196,
        b: 15,
        a: 255,
    },
    Color {
        r: 26,
        g: 188,
        b: 156,
        a: 255,
    },
    Color {
        r: 192,
        g: 57,
        b: 43,
        a: 255,
    },
    Color {
        r: 52,
        g: 73,
        b: 94,
        a: 255,
    },
    Color {
        r: 142,
        g: 68,
        b: 173,
        a: 255,
    },
];

const DEUTERANOPIA_PALETTE: &[Color] = &[
    Color {
        r: 0,
        g: 114,
        b: 178,
        a: 255,
    },
    Color {
        r: 230,
        g: 159,
        b: 0,
        a: 255,
    },
    Color {
        r: 0,
        g: 158,
        b: 115,
        a: 255,
    },
    Color {
        r: 204,
        g: 121,
        b: 167,
        a: 255,
    },
    Color {
        r: 86,
        g: 180,
        b: 233,
        a: 255,
    },
    Color {
        r: 213,
        g: 94,
        b: 0,
        a: 255,
    },
    Color {
        r: 240,
        g: 228,
        b: 66,
        a: 255,
    },
    Color {
        r: 51,
        g: 34,
        b: 136,
        a: 255,
    },
];

const PROTANOPIA_PALETTE: &[Color] = &[
    Color {
        r: 0,
        g: 114,
        b: 178,
        a: 255,
    },
    Color {
        r: 230,
        g: 159,
        b: 0,
        a: 255,
    },
    Color {
        r: 0,
        g: 158,
        b: 115,
        a: 255,
    },
    Color {
        r: 204,
        g: 121,
        b: 167,
        a: 255,
    },
    Color {
        r: 86,
        g: 180,
        b: 233,
        a: 255,
    },
    Color {
        r: 213,
        g: 94,
        b: 0,
        a: 255,
    },
    Color {
        r: 240,
        g: 228,
        b: 66,
        a: 255,
    },
    Color {
        r: 51,
        g: 34,
        b: 136,
        a: 255,
    },
];

const TRITANOPIA_PALETTE: &[Color] = &[
    Color {
        r: 230,
        g: 159,
        b: 0,
        a: 255,
    },
    Color {
        r: 86,
        g: 180,
        b: 233,
        a: 255,
    },
    Color {
        r: 0,
        g: 158,
        b: 115,
        a: 255,
    },
    Color {
        r: 204,
        g: 121,
        b: 167,
        a: 255,
    },
    Color {
        r: 213,
        g: 94,
        b: 0,
        a: 255,
    },
    Color {
        r: 240,
        g: 228,
        b: 66,
        a: 255,
    },
    Color {
        r: 0,
        g: 114,
        b: 178,
        a: 255,
    },
    Color {
        r: 51,
        g: 34,
        b: 136,
        a: 255,
    },
];

const STASH_COLOR: Color = Color {
    r: 245,
    g: 158,
    b: 11,
    a: 255,
};

fn palette_for(mode: GraphPalette) -> &'static [Color] {
    match mode {
        GraphPalette::Default => BRANCH_PALETTE,
        GraphPalette::DeuteranopiaSafe => DEUTERANOPIA_PALETTE,
        GraphPalette::ProtanopiaSafe => PROTANOPIA_PALETTE,
        GraphPalette::TritanopiaSafe => TRITANOPIA_PALETTE,
    }
}

pub struct GraphCalculator {
    commits: Vec<CommitInfo>,
    refs: HashMap<Oid, Vec<Ref>>,
    stashes: Vec<StashEntry>,
    options: GraphOptions,
}

struct CommitGraphData {
    row: usize,
    column: usize,
    is_merge: bool,
    color: Color,
}

impl GraphCalculator {
    #[must_use]
    pub fn new(
        commits: Vec<CommitInfo>,
        refs: HashMap<Oid, Vec<Ref>>,
        stashes: Vec<StashEntry>,
        options: GraphOptions,
    ) -> Self {
        Self {
            commits,
            refs,
            stashes,
            options,
        }
    }

    /// Consume the calculator and return the owned commits, avoiding a clone.
    #[must_use]
    pub fn into_commits(self) -> Vec<CommitInfo> {
        self.commits
    }

    #[must_use]
    pub fn calculate_layout(&self) -> GraphLayout {
        let commits = if self.options.hide_merges {
            self.filter_merges()
        } else {
            self.commits.clone()
        };

        if commits.is_empty() {
            return self.empty_layout();
        }

        let children = Self::build_children_map(&commits);
        let row_assignments = Self::temporal_topological_sort(&commits, &children);

        let mut graph_data: HashMap<Oid, CommitGraphData> = HashMap::new();
        let palette = palette_for(self.options.palette);
        for c in &commits {
            let row = *row_assignments.get(&c.oid).unwrap_or(&0);
            graph_data.insert(
                c.oid,
                CommitGraphData {
                    row,
                    column: 0,
                    is_merge: c.parent_oids.len() > 1,
                    color: palette[0],
                },
            );
        }

        let mut sorted: Vec<&CommitInfo> = commits.iter().collect();
        sorted.sort_by_key(|c| graph_data[&c.oid].row);

        let oid_index: HashMap<Oid, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();

        // --- gitk-style column assignment ---

        // Build displayorder: commit index by row
        let mut displayorder_idx: Vec<usize> = vec![0; commits.len()];
        for (i, c) in commits.iter().enumerate() {
            let row = *row_assignments.get(&c.oid).unwrap_or(&0);
            displayorder_idx[row] = i;
        }

        // Sort children by row ascending (youngest first) for ordertoken computation
        let children_sorted = Self::sort_children_by_row(&children, &row_assignments);

        // Compute ordertokens for stable column ordering
        let ordertokens =
            Self::compute_ordertokens(&commits, &children_sorted, &displayorder_idx, &oid_index);

        // Assign columns using row-by-row active-thread tracking (gitk algorithm)
        let (_, mut rowidlist) = Self::assign_columns(&displayorder_idx, &commits, &ordertokens);

        // Build displayorder OIDs for optimize_rows and extract_columns
        let displayorder: Vec<Oid> = displayorder_idx.iter().map(|&ci| commits[ci].oid).collect();

        // Optimize: insert padding to separate branches and fix jaggies
        optimize_rows(&mut rowidlist, &displayorder, &children_sorted);

        // Extract final columns from optimized rowidlist
        let columns = extract_columns(&rowidlist, &displayorder);
        for (oid, col) in &columns {
            if let Some(gd) = graph_data.get_mut(oid) {
                gd.column = *col;
            }
        }

        let total_columns = graph_data
            .values()
            .map(|gd| gd.column)
            .max()
            .map(|c| c + 1)
            .unwrap_or(0);

        self.assign_colors_to_nodes(&commits, &mut graph_data);
        let mut nodes = self.rebuild_nodes_with_colors(&sorted, &graph_data);
        let mut edges = self.rebuild_edges_with_colors(&sorted, &graph_data, &rowidlist);
        let (mut stash_markers, extra_cols) = self.insert_stash_nodes(&mut nodes, &mut edges);
        let total_rows = nodes
            .iter()
            .map(|n| n.row)
            .max()
            .map(|r| r + 1)
            .unwrap_or(0);
        let total_columns = total_columns + extra_cols;

        let stash_commits: Vec<CommitInfo> = stash_markers
            .iter()
            .map(|sm| {
                let stash = &self.stashes[sm.stash_index];
                CommitInfo {
                    oid: stash.oid,
                    short_oid: stash.oid.short_hex(),
                    message: stash.message.clone(),
                    summary: stash.message.clone(),
                    author: stash.author.clone(),
                    committer: stash.author.clone(),
                    author_time: stash.time,
                    commit_time: stash.time,
                    parent_oids: vec![stash.parent_oid],
                    refs: vec![],
                }
            })
            .collect();

        if self.options.orientation == GraphOrientation::BottomToTop {
            let max_row = nodes.iter().map(|n| n.row).max().unwrap_or(0);
            for node in &mut nodes {
                node.row = max_row - node.row;
            }
            for edge in &mut edges {
                let new_from = max_row - edge.from_row;
                let new_to = max_row - edge.to_row;
                edge.from_row = new_from;
                edge.to_row = new_to;
                for wp in &mut edge.waypoints {
                    wp.0 = max_row - wp.0;
                }
            }
            for marker in &mut stash_markers {
                marker.row = max_row - marker.row;
            }
        }

        GraphLayout {
            nodes,
            stash_markers,
            edges,
            total_columns,
            orientation: self.options.orientation,
            total_rows,
            stash_commits,
        }
    }

    pub fn apply_dimming(
        layout: &mut GraphLayout,
        selected_oid: Option<Oid>,
        matching_oids: Option<&HashSet<Oid>>,
    ) {
        match matching_oids {
            Some(matches) => {
                for node in &mut layout.nodes {
                    node.is_dimmed = !matches.contains(&node.oid);
                    node.is_highlighted = matches.contains(&node.oid);
                }
                for edge in &mut layout.edges {
                    edge.is_dimmed = true;
                }
            }
            None => {
                if let Some(oid) = selected_oid {
                    for node in &mut layout.nodes {
                        node.is_dimmed = node.oid != oid;
                        node.is_highlighted = node.oid == oid;
                    }
                    for edge in &mut layout.edges {
                        edge.is_dimmed = true;
                    }
                } else {
                    for node in &mut layout.nodes {
                        node.is_dimmed = false;
                        node.is_highlighted = false;
                    }
                    for edge in &mut layout.edges {
                        edge.is_dimmed = false;
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn get_ancestor_oids(&self, oid: &Oid) -> HashSet<Oid> {
        let mut ancestors = HashSet::new();
        let mut stack = vec![*oid];
        while let Some(current) = stack.pop() {
            if !ancestors.insert(current) {
                continue;
            }
            if let Some(commit) = self.commits.iter().find(|c| c.oid == current) {
                for parent in &commit.parent_oids {
                    stack.push(*parent);
                }
            }
        }
        ancestors
    }

    fn filter_merges(&self) -> Vec<CommitInfo> {
        let merge_oids: HashSet<Oid> = self
            .commits
            .iter()
            .filter(|c| c.parent_oids.len() > 1)
            .map(|c| c.oid)
            .collect();

        let mut resolve: HashMap<Oid, Oid> = HashMap::new();
        let mut changed = true;
        while changed {
            changed = false;
            for commit in &self.commits {
                if commit.parent_oids.len() <= 1 {
                    continue;
                }
                let first_parent = match commit.parent_oids.first() {
                    Some(p) => *p,
                    None => continue,
                };
                let resolved = Self::resolve_to_non_merge(&first_parent, &merge_oids, &resolve);
                if resolve.get(&commit.oid) != Some(&resolved) {
                    resolve.insert(commit.oid, resolved);
                    changed = true;
                }
            }
        }

        let mut result: Vec<CommitInfo> = self
            .commits
            .iter()
            .filter(|c| c.parent_oids.len() <= 1)
            .cloned()
            .collect();

        for c in &mut result {
            for p in &mut c.parent_oids {
                if let Some(&replacement) = resolve.get(p) {
                    *p = replacement;
                }
            }
        }

        result
    }

    fn resolve_to_non_merge(
        oid: &Oid,
        merge_oids: &HashSet<Oid>,
        resolve: &HashMap<Oid, Oid>,
    ) -> Oid {
        let mut current = *oid;
        let mut visited = HashSet::new();
        while merge_oids.contains(&current) {
            if !visited.insert(current) {
                break;
            }
            if let Some(&resolved) = resolve.get(&current) {
                current = resolved;
            } else {
                break;
            }
        }
        current
    }

    fn build_children_map(commits: &[CommitInfo]) -> HashMap<Oid, Vec<Oid>> {
        let mut children: HashMap<Oid, Vec<Oid>> = HashMap::new();
        for c in commits {
            for &p in &c.parent_oids {
                children.entry(p).or_default().push(c.oid);
            }
        }
        children
    }

    /// Compute display row order using Kahn's algorithm with a time-based
    /// priority queue (matching `git rev-list --topo-order` behavior).
    ///
    /// A commit is only assigned a row after ALL its children have been
    /// assigned rows (topological constraint). When multiple commits are
    /// available, the one with the highest commit_time (newest) is chosen.
    /// This keeps first-parent chains contiguous and avoids interleaving
    /// unrelated branches.
    fn temporal_topological_sort(
        commits: &[CommitInfo],
        children: &HashMap<Oid, Vec<Oid>>,
    ) -> HashMap<Oid, usize> {
        let n = commits.len();
        let oid_to_idx: HashMap<Oid, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();

        // Count in-degree (number of unvisited children) for each commit.
        // A commit becomes "available" when its in-degree reaches 0.
        let mut indegree: Vec<usize> = (0..n)
            .map(|i| {
                children
                    .get(&commits[i].oid)
                    .map(|kids| kids.iter().filter(|k| oid_to_idx.contains_key(*k)).count())
                    .unwrap_or(0)
            })
            .collect();

        // Max-heap by commit_time (newest first). Ties broken by oid_hex
        // (largest first — any consistent tiebreaker suffices).
        let mut heap: std::collections::BinaryHeap<(i64, String, usize)> =
            std::collections::BinaryHeap::new();

        for i in 0..n {
            if indegree[i] == 0 {
                heap.push((
                    commits[i].commit_time.timestamp(),
                    commits[i].oid.to_hex(),
                    i,
                ));
            }
        }

        let mut rows = vec![0usize; n];
        let mut row_counter = 0usize;

        while let Some((_, _, idx)) = heap.pop() {
            rows[idx] = row_counter;
            row_counter += 1;

            for parent_oid in &commits[idx].parent_oids {
                if let Some(&parent_idx) = oid_to_idx.get(parent_oid) {
                    indegree[parent_idx] -= 1;
                    if indegree[parent_idx] == 0 {
                        heap.push((
                            commits[parent_idx].commit_time.timestamp(),
                            commits[parent_idx].oid.to_hex(),
                            parent_idx,
                        ));
                    }
                }
            }
        }

        commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, rows[i]))
            .collect()
    }

    /// Sort each commit's children list by row ascending (youngest first).
    /// Required for ordertoken computation — the first child in the list is
    /// the youngest child, which determines the commit's position in the
    /// first-parent chain.
    fn sort_children_by_row(
        children: &HashMap<Oid, Vec<Oid>>,
        row_assignments: &HashMap<Oid, usize>,
    ) -> HashMap<Oid, Vec<Oid>> {
        let mut sorted = children.clone();
        for kids in sorted.values_mut() {
            kids.sort_by_key(|oid| row_assignments.get(oid).copied().unwrap_or(usize::MAX));
        }
        sorted
    }

    /// Compute ordertokens for all commits (gitk algorithm).
    ///
    /// Each commit gets a string token encoding its path from HEAD via the
    /// youngest-child chain. First parent appends `""`, second parent appends
    /// `"1"`, etc. Lexicographic comparison of tokens determines column order:
    /// the mainline (first-parent chain from HEAD) gets token `""` and sits
    /// leftmost; branches get longer tokens and go right.
    ///
    /// **Branch differentiation**: After the initial pass (which assigns `""`
    /// to all first-parent-only chains), a post-processing step differentiates
    /// branches at fork points — commits with multiple children. The first
    /// (youngest) child keeps the fork point's base token; subsequent children
    /// get their child index (`"1"`, `"2"`, …) appended to their entire
    /// descendant subtree. Without this, all non-merge branches would share
    /// token `""` and collapse into the same column.
    ///
    /// Commits are processed in row order (row 0 first) so that children
    /// always have their tokens computed before their parents.
    fn compute_ordertokens(
        commits: &[CommitInfo],
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        displayorder_idx: &[usize],
        oid_index: &HashMap<Oid, usize>,
    ) -> HashMap<Oid, String> {
        let mut ordertokens: HashMap<Oid, String> = HashMap::new();

        // Initial pass: assign base tokens via youngest-child chain.
        for &ci in displayorder_idx {
            let c = &commits[ci];
            match children_sorted.get(&c.oid).and_then(|kids| kids.first()) {
                Some(&first_child_oid) => {
                    let child_token = ordertokens
                        .get(&first_child_oid)
                        .cloned()
                        .unwrap_or_default();
                    let &child_ci = oid_index.get(&first_child_oid).unwrap();
                    let parent_idx = commits[child_ci]
                        .parent_oids
                        .iter()
                        .position(|&p| p == c.oid)
                        .unwrap_or(0);
                    let segment = if parent_idx == 0 {
                        String::new()
                    } else {
                        parent_idx.to_string()
                    };
                    ordertokens.insert(c.oid, child_token + &segment);
                }
                None => {
                    // No children — HEAD or orphan; gets base token
                    ordertokens.insert(c.oid, String::new());
                }
            }
        }

        // Post-process: differentiate branches at fork points.
        // At each fork point (commit with multiple children), the first
        // (youngest) child inherits the fork point's token. Other children
        // get a suffix (child index) appended to their entire descendant
        // subtree, ensuring each branch sorts into a distinct column.
        for &ci in displayorder_idx {
            let oid = commits[ci].oid;
            let Some(kids) = children_sorted.get(&oid) else {
                continue;
            };
            if kids.len() <= 1 {
                continue;
            }
            for (child_idx, &child_oid) in kids.iter().enumerate() {
                if child_idx == 0 {
                    continue;
                }
                let suffix = child_idx.to_string();
                propagate_branch_token(child_oid, children_sorted, &mut ordertokens, &suffix);
            }
        }

        ordertokens
    }

    /// Find the insertion position for `id` in `idlist` using a hint
    /// (gitk's idcol algorithm). When the new element's ordertoken equals
    /// the token at the hint position, the new element is inserted AT the
    /// hint (pushing existing elements right), so the first parent inherits
    /// its child's column.
    fn idcol(idlist: &[Oid], id: Oid, ordertokens: &HashMap<Oid, String>, hint: usize) -> usize {
        if idlist.is_empty() {
            return 0;
        }

        let t = ordertokens.get(&id).map(String::as_str).unwrap_or("");

        if hint >= idlist.len() {
            for p in (0..idlist.len()).rev() {
                let token_p = ordertokens
                    .get(&idlist[p])
                    .map(String::as_str)
                    .unwrap_or("");
                if t >= token_p {
                    return p + 1;
                }
            }
            return 0;
        }

        let token_at_hint = ordertokens
            .get(&idlist[hint])
            .map(String::as_str)
            .unwrap_or("");

        if t < token_at_hint {
            for p in (0..hint).rev() {
                let token_p = ordertokens
                    .get(&idlist[p])
                    .map(String::as_str)
                    .unwrap_or("");
                if t >= token_p {
                    return p + 1;
                }
            }
            0
        } else if t > token_at_hint {
            let mut j = hint + 1;
            while j < idlist.len() {
                let token_j = ordertokens
                    .get(&idlist[j])
                    .map(String::as_str)
                    .unwrap_or("");
                if t >= token_j {
                    j += 1;
                } else {
                    break;
                }
            }
            j
        } else {
            hint
        }
    }

    /// Assign columns using gitk's row-by-row active-thread tracking.
    ///
    /// Returns (columns, rowidlist). The rowidlist stores the full active-
    /// thread list at each row, needed by optimize_rows and thread tracing.
    fn assign_columns(
        displayorder_idx: &[usize],
        commits: &[CommitInfo],
        ordertokens: &HashMap<Oid, String>,
    ) -> (HashMap<Oid, usize>, Vec<Vec<Option<Oid>>>) {
        let n = displayorder_idx.len();
        if n == 0 {
            return (HashMap::new(), Vec::new());
        }

        let mut idlist: Vec<Oid> = Vec::with_capacity(32);
        let mut columns: HashMap<Oid, usize> = HashMap::new();
        let mut rowidlist: Vec<Vec<Option<Oid>>> = Vec::with_capacity(n);

        // Row 0: just the HEAD commit
        let head_oid = commits[displayorder_idx[0]].oid;
        idlist.push(head_oid);
        columns.insert(head_oid, 0);
        rowidlist.push(idlist.iter().map(|&o| Some(o)).collect());

        for row in 1..n {
            let prev_ci = displayorder_idx[row - 1];
            let curr_ci = displayorder_idx[row];
            let prev = &commits[prev_ci];
            let curr = &commits[curr_ci];

            // Remove prev from idlist (it's been drawn)
            let removed_col = idlist.iter().position(|&x| x == prev.oid);
            if let Some(col) = removed_col {
                let _removed = idlist.remove(col);
            }

            // Add prev's parents using removed column as hint (gitk algorithm).
            let mut hint = removed_col.unwrap_or(0);
            for &p_oid in &prev.parent_oids {
                if ordertokens.contains_key(&p_oid) && !idlist.contains(&p_oid) {
                    let col = Self::idcol(&idlist, p_oid, ordertokens, hint);
                    idlist.insert(col, p_oid);
                    hint = col;
                }
            }

            // Ensure curr is in idlist. Use end-of-list as hint so newly
            // appearing branch tips don't displace established threads.
            if !idlist.contains(&curr.oid) {
                let col = Self::idcol(&idlist, curr.oid, ordertokens, idlist.len());
                idlist.insert(col, curr.oid);
            }

            // Record column of curr
            let col = idlist.iter().position(|&x| x == curr.oid).unwrap();
            columns.insert(curr.oid, col);

            // Store full rowidlist for optimize_rows + thread tracing
            rowidlist.push(idlist.iter().map(|&o| Some(o)).collect());
        }

        (columns, rowidlist)
    }

    fn insert_stash_nodes(
        &self,
        nodes: &mut Vec<NodePosition>,
        edges: &mut Vec<Edge>,
    ) -> (Vec<StashMarker>, usize) {
        let node_map: HashMap<Oid, &NodePosition> = nodes.iter().map(|n| (n.oid, n)).collect();
        let mut by_parent: Vec<(usize, &StashEntry, usize, usize)> = Vec::new();
        for (idx, stash) in self.stashes.iter().enumerate() {
            if let Some(parent) = node_map.get(&stash.parent_oid) {
                by_parent.push((idx, stash, parent.row, parent.column));
            }
        }
        by_parent.sort_by_key(|(_, _, row, _)| *row);

        let base_max_col = nodes.iter().map(|n| n.column).max().unwrap_or(0);
        let mut extra_cols: usize = 0;
        let mut parent_branch_col: HashMap<Oid, usize> = HashMap::new();

        let mut stash_markers: Vec<StashMarker> = Vec::new();
        for (idx, stash, parent_row, parent_col) in &by_parent {
            let after_parent = stash_markers
                .iter()
                .filter(|sm: &&StashMarker| sm.parent_oid == stash.parent_oid)
                .count();
            let insert_row = parent_row + after_parent;

            let branch_col = match parent_branch_col.entry(stash.parent_oid) {
                std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    extra_cols += 1;
                    let col = base_max_col + extra_cols;
                    e.insert(col);
                    col
                }
            };

            for node in nodes.iter_mut() {
                if node.row >= insert_row {
                    node.row += 1;
                }
            }
            for edge in edges.iter_mut() {
                if edge.from_row >= insert_row {
                    edge.from_row += 1;
                }
                if edge.to_row >= insert_row {
                    edge.to_row += 1;
                }
            }

            let parent_new_row = nodes
                .iter()
                .find(|n| n.oid == stash.parent_oid)
                .map(|n| n.row)
                .unwrap_or(*parent_row + 1);

            nodes.push(NodePosition {
                oid: stash.oid,
                row: insert_row,
                column: branch_col,
                is_merge: false,
                is_stash: true,
                color: STASH_COLOR,
                is_dimmed: false,
                is_highlighted: false,
            });

            edges.push(Edge {
                from_row: insert_row,
                from_col: branch_col,
                to_row: parent_new_row,
                to_col: *parent_col,
                edge_type: EdgeType::Straight,
                color: STASH_COLOR,
                is_dimmed: false,
                edge_style: EdgeStyle::Dashed,
                waypoints: Vec::new(),
            });

            stash_markers.push(StashMarker {
                row: insert_row,
                column: branch_col,
                stash_index: *idx,
                stash_oid: stash.oid,
                parent_oid: stash.parent_oid,
                message: stash.message.clone(),
            });
        }

        let node_row_map: HashMap<Oid, usize> = nodes.iter().map(|n| (n.oid, n.row)).collect();
        for marker in &mut stash_markers {
            if let Some(&row) = node_row_map.get(&marker.stash_oid) {
                marker.row = row;
            }
        }

        (stash_markers, extra_cols)
    }

    fn assign_colors_to_nodes(
        &self,
        commits: &[CommitInfo],
        graph_data: &mut HashMap<Oid, CommitGraphData>,
    ) {
        let palette = palette_for(self.options.palette);
        let mut color_idx = 0usize;

        if self.options.color_mode == GraphColorMode::ByBranch {
            let mut lane_colors: HashMap<usize, Color> = HashMap::new();
            let mut ref_colors: HashMap<String, Color> = HashMap::new();
            for c in commits {
                let col = graph_data.get(&c.oid).map(|gd| gd.column).unwrap_or(0);
                let color = *lane_colors.entry(col).or_insert_with(|| {
                    let clr = palette[color_idx % palette.len()];
                    color_idx += 1;
                    clr
                });
                if let Some(refs) = self.refs.get(&c.oid) {
                    for r in refs {
                        let name = match r {
                            Ref::Branch(b) => b.name.clone(),
                            Ref::Tag(t) => t.name.clone(),
                            _ => continue,
                        };
                        ref_colors.entry(name).or_insert(color);
                    }
                }
                if let Some(gd) = graph_data.get_mut(&c.oid) {
                    gd.color = color;
                }
            }
        } else {
            let mut author_colors: HashMap<String, Color> = HashMap::new();
            for c in commits {
                let author_key = format!("{} <{}>", c.author.name, c.author.email);
                let color = *author_colors.entry(author_key).or_insert_with(|| {
                    let clr = palette[color_idx % palette.len()];
                    color_idx += 1;
                    clr
                });
                if let Some(gd) = graph_data.get_mut(&c.oid) {
                    gd.color = color;
                }
            }
        }
    }

    fn rebuild_nodes_with_colors(
        &self,
        sorted: &[&CommitInfo],
        graph_data: &HashMap<Oid, CommitGraphData>,
    ) -> Vec<NodePosition> {
        sorted
            .iter()
            .map(|c| {
                let gd = &graph_data[&c.oid];
                NodePosition {
                    oid: c.oid,
                    row: gd.row,
                    column: gd.column,
                    is_merge: gd.is_merge,
                    is_stash: false,
                    color: gd.color,
                    is_dimmed: false,
                    is_highlighted: false,
                }
            })
            .collect()
    }

    fn rebuild_edges_with_colors(
        &self,
        sorted: &[&CommitInfo],
        graph_data: &HashMap<Oid, CommitGraphData>,
        rowidlist: &[Vec<Option<Oid>>],
    ) -> Vec<Edge> {
        let mut edges = Vec::new();
        for c in sorted {
            let c_row = graph_data[&c.oid].row;
            let c_col = graph_data[&c.oid].column;
            let c_color = graph_data[&c.oid].color;
            for (pi, &p_oid) in c.parent_oids.iter().enumerate() {
                if let Some(p_gd) = graph_data.get(&p_oid) {
                    let edge_type = if pi == 0 {
                        if c_col == p_gd.column {
                            EdgeType::Straight
                        } else {
                            EdgeType::Branch
                        }
                    } else {
                        EdgeType::Merge
                    };
                    let edge_style = if self.options.palette == GraphPalette::Default {
                        EdgeStyle::Solid
                    } else {
                        match c_col % 3 {
                            0 => EdgeStyle::Solid,
                            1 => EdgeStyle::Dashed,
                            _ => EdgeStyle::Dotted,
                        }
                    };

                    // Trace the parent's thread through the rowidlist to
                    // find direction-change waypoints (gitk drawlineseg approach).
                    let waypoints = trace_thread(p_oid, c_row, p_gd.row, rowidlist);

                    edges.push(Edge {
                        from_row: c_row,
                        from_col: c_col,
                        to_row: p_gd.row,
                        to_col: p_gd.column,
                        edge_type,
                        color: c_color,
                        is_dimmed: false,
                        edge_style,
                        waypoints,
                    });
                }
            }
        }
        edges
    }

    fn empty_layout(&self) -> GraphLayout {
        GraphLayout {
            nodes: Vec::new(),
            stash_markers: Vec::new(),
            edges: Vec::new(),
            total_columns: 0,
            orientation: self.options.orientation,
            total_rows: 0,
            stash_commits: Vec::new(),
        }
    }
}

/// Recursively append `suffix` to the ordertoken of `start_oid` and all its
/// descendants in the children map.
///
/// This is used during ordertoken post-processing: when a fork point has
/// children [A, B, C], B's subtree gets `"1"` and C's subtree gets `"2"`,
/// ensuring each branch sorts into a distinct column via `idcol`.
fn propagate_branch_token(
    start_oid: Oid,
    children_sorted: &HashMap<Oid, Vec<Oid>>,
    ordertokens: &mut HashMap<Oid, String>,
    suffix: &str,
) {
    let mut stack = vec![start_oid];
    let mut visited = HashSet::new();
    while let Some(oid) = stack.pop() {
        if !visited.insert(oid) {
            continue;
        }
        if let Some(token) = ordertokens.get_mut(&oid) {
            token.push_str(suffix);
        }
        if let Some(kids) = children_sorted.get(&oid) {
            for &kid in kids.iter().rev() {
                stack.push(kid);
            }
        }
    }
}

/// Trace a thread (parent_oid) through the rowidlist from child_row to
/// parent_row, recording (row, col) at each direction change.
///
/// This matches gitk's drawlineseg approach: walk consecutive rows where
/// the thread appears, recording coordinates only when the column delta
/// changes direction. This produces the minimal set of waypoints needed
/// to render the thread's actual path.
fn trace_thread(
    parent_oid: Oid,
    child_row: usize,
    parent_row: usize,
    rowidlist: &[Vec<Option<Oid>>],
) -> Vec<(usize, usize)> {
    if parent_row <= child_row + 1 {
        return Vec::new();
    }

    // Collect the thread's column at each intermediate row
    let mut path: Vec<(usize, usize)> = Vec::new();
    for r in (child_row + 1)..parent_row {
        if r >= rowidlist.len() {
            break;
        }
        if let Some(col) = rowidlist[r].iter().position(|&x| x == Some(parent_oid)) {
            path.push((r, col));
        }
    }

    if path.is_empty() {
        return Vec::new();
    }

    // Simplify: keep only waypoints where direction changes
    // (matching gitk's drawlineseg coordinate generation)
    let mut waypoints = Vec::new();
    let mut prev_dir: i64 = 0;

    for &(r, col) in &path {
        if waypoints.is_empty() {
            waypoints.push((r, col));
            continue;
        }
        let (_, prev_col) = waypoints.last().unwrap();
        let dir = col as i64 - *prev_col as i64;
        if dir != prev_dir {
            // Direction changed — the last waypoint is the turning point
            // But we already added it. Update prev_dir.
            prev_dir = dir;
        }
        waypoints.push((r, col));
    }

    // Further simplify: remove intermediate waypoints with same direction
    let mut simplified: Vec<(usize, usize)> = Vec::new();
    for wp in waypoints {
        while simplified.len() >= 2 {
            let n = simplified.len();
            let (_, c0) = simplified[n - 2];
            let (_, c1) = simplified[n - 1];
            let d0 = c1 as i64 - c0 as i64;
            let d1 = wp.1 as i64 - c1 as i64;
            if d0 == d1 {
                simplified.pop();
            } else {
                break;
            }
        }
        simplified.push(wp);
    }

    simplified
}

// --- optimize_rows (gitk port) ---

/// Insert `npad` padding slots (None) at position `col` in `rowidlist[row]`.
/// Tries to absorb one existing None from positions after `col` to avoid
/// unbounded list growth (matches gitk's insert_pad behavior).
fn insert_pad(rowidlist: &mut [Vec<Option<Oid>>], row: usize, col: usize, npad: usize) {
    let idlist = &mut rowidlist[row];

    // Try to absorb one existing None from positions > col
    let absorb_idx: Option<usize> = (col + 1..idlist.len()).find(|&i| idlist[i].is_none());

    let mut new_list = Vec::with_capacity(idlist.len() + npad);
    new_list.extend_from_slice(&idlist[..col]);
    new_list.extend(std::iter::repeat_n(None, npad));

    if let Some(ai) = absorb_idx {
        new_list.extend_from_slice(&idlist[col..ai]);
        new_list.extend_from_slice(&idlist[ai + 1..]);
    } else {
        new_list.extend_from_slice(&idlist[col..]);
    }

    *idlist = new_list;
}

/// Optimize the rowidlist by inserting padding to prevent lines from moving
/// more than 1 column between consecutive rows, and to fix zigzags.
///
/// This is a port of gitk's optimize_rows (lines 5871-5993). It fixes the
/// "branch collapse" caused by hint-based idcol — where two siblings share
/// the same column — by inserting padding to separate them.
fn optimize_rows(
    rowidlist: &mut Vec<Vec<Option<Oid>>>,
    displayorder: &[Oid],
    children: &HashMap<Oid, Vec<Oid>>,
) {
    let n = rowidlist.len();
    if n < 3 {
        return;
    }
    let mut rowisopt = vec![false; n];
    optimize_rows_impl(rowidlist, &mut rowisopt, displayorder, children, 1, n);
}

fn optimize_rows_impl(
    rowidlist: &mut Vec<Vec<Option<Oid>>>,
    rowisopt: &mut [bool],
    displayorder: &[Oid],
    children: &HashMap<Oid, Vec<Oid>>,
    start_row: usize,
    end_row: usize,
) {
    let mut row = start_row;
    let mut col;

    while row < end_row {
        if rowisopt[row] {
            row += 1;
            continue;
        }
        if row < 2 {
            rowisopt[row] = true;
            row += 1;
            continue;
        }

        let y0 = row - 1;
        let ym = row - 2;

        if rowidlist[row].is_empty() || rowidlist[y0].is_empty() || rowidlist[ym].is_empty() {
            rowisopt[row] = true;
            row += 1;
            continue;
        }

        let mut haspad = false;
        col = 0;

        while col < rowidlist[row].len() {
            let id_opt = rowidlist[row][col];

            // Line goes straight up?
            if col < rowidlist[y0].len() && rowidlist[y0][col] == id_opt {
                col += 1;
                continue;
            }

            let id = match id_opt {
                Some(o) => o,
                None => {
                    haspad = true;
                    col += 1;
                    continue;
                }
            };

            // Find id in previous row
            let x0 = match rowidlist[y0].iter().position(|&x| x == Some(id)) {
                Some(x) => x,
                None => {
                    col += 1;
                    continue;
                }
            };

            let z = x0 as i64 - col as i64;

            // Check isarrow
            let mut isarrow = false;
            let mut z0: Option<i64> = None;

            if let Some(xm) = rowidlist[ym].iter().position(|&x| x == Some(id)) {
                z0 = Some(xm as i64 - x0 as i64);
            }

            if z0.is_none() {
                // If commit at y0 is NOT the first child of id, it's an arrow
                let first_child = children.get(&id).and_then(|kids| kids.first());
                if first_child != Some(&displayorder[y0]) {
                    isarrow = true;
                }
            }

            if !isarrow
                && id != displayorder[row]
                && row + 1 < rowidlist.len()
                && !rowidlist[row + 1].contains(&Some(id))
            {
                isarrow = true;
            }

            // Fix lines going left too much
            if z < -1 || (z < 0 && isarrow) {
                let npad = (-1 - z + isarrow as i64) as usize;
                insert_pad(rowidlist, y0, x0, npad);
                if y0 > 0 {
                    rowisopt[y0] = false;
                    optimize_rows_impl(rowidlist, rowisopt, displayorder, children, y0, row);
                }
                // Re-read positions after padding changes
                continue;
            }

            // Fix lines going right too much
            if z > 1 || (z > 0 && isarrow) {
                let npad = (z - 1 + isarrow as i64) as usize;
                insert_pad(rowidlist, row, col, npad);
                haspad = true;
                col += npad;
                continue;
            }

            // Avoid jigging left then immediately right
            if let Some(z0v) = z0
                && z < 0
                && z0v > 0
            {
                insert_pad(rowidlist, y0, x0, 1);
                if y0 > 0 {
                    rowisopt[y0] = false;
                    optimize_rows_impl(rowidlist, rowisopt, displayorder, children, y0, row);
                }
                continue;
            }

            col += 1;
        }

        // If no padding was added, insert one pad for visual clarity
        if !haspad {
            let idlist_len = rowidlist[row].len();

            // Find the first column (from right) that doesn't have a line going right.
            // gitk: after finding, incr col then check if < llength.
            let mut found_col: Option<usize> = None;
            for c in (0..idlist_len).rev() {
                let cid = rowidlist[row][c];
                if cid.is_none() {
                    found_col = Some(c);
                    break;
                }
                let cx0 = rowidlist[y0].iter().position(|&x| x == cid);
                let x0 = match cx0 {
                    Some(x) => x as i64,
                    None => {
                        let kid = displayorder[y0];
                        let kid_col = rowidlist[y0].iter().position(|&x| x == Some(kid));
                        if let Some(kc) = kid_col {
                            kc as i64
                        } else {
                            continue;
                        }
                    }
                };
                if x0 <= c as i64 {
                    found_col = Some(c);
                    break;
                }
            }

            // gitk: if {$x0 >= 0 && [incr col] < [llength $idlist]}
            if let Some(c) = found_col {
                let insert_at = c + 1;
                if insert_at < idlist_len {
                    rowidlist[row].insert(insert_at, None);
                }
            }
        }
        rowisopt[row] = true;
        row += 1;
    }
}

/// Extract each commit's final column from the optimized rowidlist.
fn extract_columns(rowidlist: &[Vec<Option<Oid>>], displayorder: &[Oid]) -> HashMap<Oid, usize> {
    let mut columns = HashMap::new();
    for (row, idlist) in rowidlist.iter().enumerate() {
        if row >= displayorder.len() {
            break;
        }
        let oid = displayorder[row];
        if let Some(col) = idlist.iter().position(|&x| x == Some(oid)) {
            columns.insert(oid, col);
        }
    }
    columns
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_oid(n: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = n;
        Oid::from_bytes(bytes)
    }

    fn make_commit(oid: u8, parents: Vec<u8>, msg: &str) -> CommitInfo {
        CommitInfo {
            oid: make_oid(oid),
            short_oid: format!("{oid:02x}00000"),
            message: msg.to_string(),
            summary: msg.to_string(),
            author: Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
            },
            committer: Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
            },
            author_time: Utc.timestamp_opt(1000 + oid as i64, 0).single().unwrap(),
            commit_time: Utc.timestamp_opt(1000 + oid as i64, 0).single().unwrap(),
            parent_oids: parents.into_iter().map(make_oid).collect(),
            refs: Vec::new(),
        }
    }

    #[test]
    fn linear_graph_single_column() {
        let c1 = make_commit(1, vec![], "first");
        let c2 = make_commit(2, vec![1], "second");
        let c3 = make_commit(3, vec![2], "third");
        let commits = vec![c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 3);
        assert_eq!(layout.total_columns, 1);
        for node in &layout.nodes {
            assert_eq!(node.column, 0);
        }
    }

    #[test]
    fn branch_uses_two_columns() {
        let c1 = make_commit(1, vec![], "initial");
        let c2 = make_commit(2, vec![1], "on main");
        let c3 = make_commit(3, vec![1], "on branch");
        let commits = vec![c2, c3, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 3);
        // optimize_rows should separate the two branches
        let columns: HashSet<usize> = layout.nodes.iter().map(|n| n.column).collect();
        assert!(
            columns.len() >= 2,
            "branching should use at least 2 columns after optimize_rows, got {:?}: {:?}",
            columns,
            layout
                .nodes
                .iter()
                .map(|n| (n.oid.short_hex(), n.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn property_topological_order() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "second");
        let c3 = make_commit(3, vec![2], "third");
        let c4 = make_commit(4, vec![3], "fourth");
        let commits = vec![c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        for edge in &layout.edges {
            assert!(
                edge.from_row < edge.to_row,
                "parent (to_row={}) should have higher row than child (from_row={})",
                edge.to_row,
                edge.from_row
            );
        }
    }

    #[test]
    fn property_completeness() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let c3 = make_commit(3, vec![1], "c");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];
        let parent_count: usize = commits.iter().map(|c| c.parent_oids.len()).sum();
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 4, "one node per commit");
        assert_eq!(layout.edges.len(), parent_count, "one edge per parent link");
    }

    #[test]
    fn property_branch_continuity() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "a");
        let c3 = make_commit(3, vec![2], "b");
        let c4 = make_commit(4, vec![3], "c");
        let commits = vec![c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        let columns: Vec<usize> = layout.nodes.iter().map(|n| n.column).collect();
        assert!(
            columns.iter().all(|&c| c == columns[0]),
            "linear chain should stay in same column (branch continuity)"
        );
    }

    #[test]
    fn hide_merges_removes_merge_commits() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "branch a");
        let c3 = make_commit(3, vec![1], "branch b");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];
        let options = GraphOptions {
            hide_merges: true,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.nodes.iter().all(|n| !n.is_merge));
    }

    #[test]
    fn hide_merges_rewires_child_of_merge() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "branch a");
        let c3 = make_commit(3, vec![1], "branch b");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let c5 = make_commit(5, vec![4], "after merge");
        let commits = vec![c5, c4, c3, c2, c1];
        let options = GraphOptions {
            hide_merges: true,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 4, "merge removed, 4 nodes remain");
        assert!(layout.nodes.iter().all(|n| !n.is_merge));
        let after_merge = layout.nodes.iter().find(|n| n.oid == make_oid(5)).unwrap();
        let edges_to_after: Vec<_> = layout
            .edges
            .iter()
            .filter(|e| e.to_row == after_merge.row || e.from_row == after_merge.row)
            .collect();
        assert!(
            edges_to_after.iter().any(|e| {
                let other_oid = layout.nodes.iter().find(|n| {
                    n.row
                        == if e.from_row == after_merge.row {
                            e.to_row
                        } else {
                            e.from_row
                        }
                });
                other_oid.is_some_and(|n| n.oid == make_oid(2))
            }),
            "child of merge should be connected to merge's first parent"
        );
    }

    #[test]
    fn bottom_to_top_reverses_rows() {
        let c1 = make_commit(1, vec![], "first");
        let c2 = make_commit(2, vec![1], "second");
        let c3 = make_commit(3, vec![2], "third");
        let commits = vec![c3, c2, c1];
        let options = GraphOptions {
            orientation: GraphOrientation::BottomToTop,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        let mut nodes_by_oid: HashMap<Oid, &NodePosition> = HashMap::new();
        for n in &layout.nodes {
            nodes_by_oid.insert(n.oid, n);
        }
        assert!(
            nodes_by_oid[&make_oid(3)].row > nodes_by_oid[&make_oid(1)].row,
            "in bottom-to-top, newest commit (3) should have higher row (at bottom)"
        );
    }

    #[test]
    fn viewport_filters_nodes_and_edges() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let c3 = make_commit(3, vec![2], "c");
        let c4 = make_commit(4, vec![3], "d");
        let c5 = make_commit(5, vec![4], "e");
        let commits = vec![c5, c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        let vp = layout.from_visible_range(1, 3);
        assert!(vp.nodes.len() <= 2);
        assert_eq!(vp.rows, 1..3);
    }

    #[test]
    fn viewport_retains_spanning_edges() {
        // Topology that produces a long-spanning edge:
        //   c1 (root, row 4)
        //   c2 (root, row 3)
        //   c3 (root, row 2)
        //   c4 (child of c1, row 1)
        //   c5 (merge of c2+c4, row 0)
        // Edge c5→c2 spans from row 0 to row 3 — both outside viewport [1,3).
        let c1 = make_commit(1, vec![], "root1");
        let c2 = make_commit(2, vec![], "root2");
        let c3 = make_commit(3, vec![], "root3");
        let c4 = make_commit(4, vec![1], "branch from c1");
        let c5 = make_commit(5, vec![2, 4], "merge c2+c4");
        let commits = vec![c5, c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let vp = layout.from_visible_range(1, 3);
        assert!(
            vp.edges.iter().any(|e| e.from_row < 1 && e.to_row >= 3),
            "spanning edge (both endpoints outside viewport) should be retained"
        );
    }

    #[test]
    fn empty_commits_produce_empty_layout() {
        let calc = GraphCalculator::new(
            Vec::new(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        assert!(layout.nodes.is_empty());
        assert!(layout.edges.is_empty());
        assert_eq!(layout.total_rows, 0);
    }

    #[test]
    fn apply_dimming_highlights_matching() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let commits = vec![c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let mut layout = calc.calculate_layout();
        let matching: HashSet<Oid> = [make_oid(2)].into_iter().collect();
        GraphCalculator::apply_dimming(&mut layout, None, Some(&matching));
        let n1 = layout.nodes.iter().find(|n| n.oid == make_oid(1)).unwrap();
        let n2 = layout.nodes.iter().find(|n| n.oid == make_oid(2)).unwrap();
        assert!(n1.is_dimmed);
        assert!(!n1.is_highlighted);
        assert!(!n2.is_dimmed);
        assert!(n2.is_highlighted);
    }

    #[test]
    fn color_by_author_mode() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let commits = vec![c2, c1];
        let options = GraphOptions {
            color_mode: GraphColorMode::ByAuthor,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        let colors: std::collections::HashSet<Color> =
            layout.nodes.iter().map(|n| n.color).collect();
        assert!(
            !colors.is_empty(),
            "color-by-author should assign colors to nodes"
        );
    }

    #[test]
    fn merge_child_has_no_spurious_edge_to_merge_side() {
        // Topology from bug report: 5701a67 (parents: 59295b8, ed13a61),
        // 59295b8 (parents: 2f38767, 5cb5908).
        // 59295b8 must have exactly 2 out-edges (to its own parents),
        // not a spurious third edge to ed13a61.
        //
        // Layout:
        //   row 0: 5701a67 (merge of 59295b8 + ed13a61)
        //   row 1: 59295b8 (merge of 2f38767 + 5cb5908)
        //   row 2: ed13a61
        //   row 3: 5cb5908
        //   row 4: 2f38767
        //   row 5-7: roots
        let roots = vec![
            make_commit(1, vec![], "root for 2f38767"),
            make_commit(2, vec![], "root for 5cb5908"),
            make_commit(3, vec![], "root for ed13a61"),
        ];
        let c4 = make_commit(4, vec![1], "2f38767");
        let c5 = make_commit(5, vec![2], "5cb5908");
        let c6 = make_commit(6, vec![3], "ed13a61");
        let c7 = make_commit(7, vec![4, 5], "59295b8");
        let c8 = make_commit(8, vec![7, 6], "5701a67");
        let commits = vec![c8, c7, c6, c5, c4];
        let mut all = roots;
        all.extend(commits);
        let calc = GraphCalculator::new(all, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let commit_59295b8 = make_oid(7);
        let commit_ed13a61 = make_oid(6);

        // Find 59295b8's node
        let n7 = layout
            .nodes
            .iter()
            .find(|n| n.oid == commit_59295b8)
            .expect("59295b8 must be in layout");

        // Find all edges from 59295b8
        let edges_from_59295b8: Vec<&Edge> = layout
            .edges
            .iter()
            .filter(|e| e.from_row == n7.row && e.from_col == n7.column)
            .collect();

        // 59295b8 must have exactly 2 out-edges (to 2f38767 and 5cb5908)
        assert_eq!(
            edges_from_59295b8.len(),
            2,
            "59295b8 must have exactly 2 out-edges, got {}",
            edges_from_59295b8.len()
        );

        // Verify no edge from 59295b8 to ed13a61
        for edge in &edges_from_59295b8 {
            let to_node = layout
                .nodes
                .iter()
                .find(|n| n.row == edge.to_row && n.column == edge.to_col);
            if let Some(to_node) = to_node {
                assert_ne!(
                    to_node.oid, commit_ed13a61,
                    "59295b8 must not have an edge to ed13a61"
                );
            }
        }

        // Verify 5701a67 has edges to both 59295b8 and ed13a61
        let n8 = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(8))
            .expect("5701a67 must be in layout");
        let edges_from_5701a67: Vec<&Edge> = layout
            .edges
            .iter()
            .filter(|e| e.from_row == n8.row && e.from_col == n8.column)
            .collect();
        assert_eq!(
            edges_from_5701a67.len(),
            2,
            "5701a67 must have exactly 2 out-edges"
        );

        // Verify 59295b8's two parents (2f38767, 5cb5908) have distinct columns
        let commit_2f38767 = make_oid(4);
        let commit_5cb5908 = make_oid(5);
        let n4 = layout
            .nodes
            .iter()
            .find(|n| n.oid == commit_2f38767)
            .expect("2f38767 must be in layout");
        let n5 = layout
            .nodes
            .iter()
            .find(|n| n.oid == commit_5cb5908)
            .expect("5cb5908 must be in layout");
        assert_ne!(
            n4.column, n5.column,
            "59295b8's two parents must be in different columns, got {} and {}",
            n4.column, n5.column
        );
    }

    #[test]
    fn property_merge_parents_have_distinct_columns() {
        let test_cases = vec![
            vec![
                make_commit(1, vec![], "root1"),
                make_commit(2, vec![], "root2"),
                make_commit(3, vec![], "root3"),
                make_commit(4, vec![1], "2f38767"),
                make_commit(5, vec![2], "5cb5908"),
                make_commit(6, vec![3], "ed13a61"),
                make_commit(7, vec![4, 5], "59295b8"),
                make_commit(8, vec![7, 6], "5701a67"),
            ],
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "main"),
                make_commit(3, vec![1], "branch"),
                make_commit(4, vec![2, 3], "merge"),
            ],
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![1], "b"),
                make_commit(4, vec![2, 3], "merge1"),
                make_commit(5, vec![4], "c"),
                make_commit(6, vec![1], "d"),
                make_commit(7, vec![5, 6], "merge2"),
            ],
        ];

        for commits in &test_cases {
            let calc = GraphCalculator::new(
                commits.clone(),
                HashMap::new(),
                Vec::new(),
                GraphOptions::default(),
            );
            let layout = calc.calculate_layout();

            let node_by_oid: HashMap<Oid, &NodePosition> =
                layout.nodes.iter().map(|n| (n.oid, n)).collect();

            for commit in &layout.nodes {
                let orig = match commits.iter().find(|c| c.oid == commit.oid) {
                    Some(c) => c,
                    None => continue,
                };
                if orig.parent_oids.len() < 2 {
                    continue;
                }
                let first_parent = match node_by_oid.get(&orig.parent_oids[0]) {
                    Some(n) => n,
                    None => continue,
                };
                for parent_oid in orig.parent_oids[1..].iter() {
                    if let Some(other_parent) = node_by_oid.get(parent_oid) {
                        assert_ne!(
                            first_parent.column, other_parent.column,
                            "Merge commit {} has two parents in column {}",
                            orig.short_oid, first_parent.column,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn property_column_count_reasonable_for_complex_merge() {
        let commits = vec![
            make_commit(1, vec![], "root1"),
            make_commit(2, vec![], "root2"),
            make_commit(3, vec![], "root3"),
            make_commit(4, vec![1], "2f38767"),
            make_commit(5, vec![2], "5cb5908"),
            make_commit(6, vec![3], "ed13a61"),
            make_commit(7, vec![4, 5], "59295b8"),
            make_commit(8, vec![7, 6], "5701a67"),
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // This topology has 3 independent branches merging into one mainline.
        // gitk-style layout should use at most 3 columns (mainline + 2 branch lanes).
        assert!(
            layout.total_columns <= 3,
            "Complex merge should use at most 3 columns, got {}",
            layout.total_columns
        );
    }

    /// Verify the parent-preservation invariant: every non-merge commit that
    /// has ≥1 parent when merges are shown must still have ≥1 parent edge
    /// when merges are hidden.
    fn assert_parents_preserved(show_layout: &GraphLayout, hide_layout: &GraphLayout, label: &str) {
        let show_oid_to_node: HashMap<Oid, &NodePosition> =
            show_layout.nodes.iter().map(|n| (n.oid, n)).collect();

        for node in &hide_layout.nodes {
            let show_node = match show_oid_to_node.get(&node.oid) {
                Some(n) => *n,
                None => continue,
            };

            let had_parents = show_layout
                .edges
                .iter()
                .any(|e| e.from_row == show_node.row && e.from_col == show_node.column);

            if !had_parents {
                continue;
            }

            let has_parents = hide_layout
                .edges
                .iter()
                .any(|e| e.from_row == node.row && e.from_col == node.column);

            assert!(
                has_parents,
                "{label}: node {} had parents in show-layout but has none in hide-layout",
                node.oid.short_hex(),
            );
        }
    }

    #[test]
    fn property_hide_merges_preserves_parents_simple() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let m = make_commit(3, vec![1, 2], "merge");
        let c4 = make_commit(4, vec![3], "after merge");
        let commits = vec![c4, m, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "simple merge");
    }

    #[test]
    fn property_hide_merges_preserves_parents_chained() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let c3 = make_commit(3, vec![], "root C");
        let m1 = make_commit(4, vec![1, 2], "inner merge");
        let m2 = make_commit(5, vec![4, 3], "outer merge");
        let c6 = make_commit(6, vec![5], "after outer merge");
        let commits = vec![c6, m2, m1, c3, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "chained merges");
    }

    #[test]
    fn property_hide_merges_preserves_parents_multiple_children() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let m = make_commit(3, vec![1, 2], "merge");
        let c4 = make_commit(4, vec![3], "child 1");
        let c5 = make_commit(5, vec![3], "child 2");
        let commits = vec![c5, c4, m, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "multiple children");
    }

    #[test]
    fn property_hide_merges_preserves_parents_only_merge_parent() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let m = make_commit(3, vec![1, 2], "merge");
        let c4 = make_commit(4, vec![3], "only merge parent");
        let c5 = make_commit(5, vec![4], "grandchild");
        let commits = vec![c5, c4, m, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "only merge parent");
    }

    #[test]
    fn property_hide_merges_preserves_parents_merge_of_merge() {
        // c1 → c2 → m1 ← c3
        //              ↓
        //      m2 ← c4
        //       ↓
        //      c5
        // m1 is a merge, m2 is a merge whose first parent is m1
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "main line");
        let c3 = make_commit(3, vec![1], "branch");
        let m1 = make_commit(4, vec![2, 3], "inner merge");
        let c4 = make_commit(5, vec![1], "feature");
        let m2 = make_commit(6, vec![4, 5], "outer merge");
        let c7 = make_commit(7, vec![6], "after outer merge");
        let commits = vec![c7, m2, m1, c4, c3, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "merge of merge");
    }

    #[test]
    fn compaction_reduces_column_count() {
        // Two non-overlapping branches should share a column after compaction.
        // Topology: root → a1 → merge1 → b1 → merge2
        //                  f1 →┘         f2 →┘
        // f1 and f2 are active at different times (no row overlap).
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "a1");
        let c3 = make_commit(3, vec![1], "f1");
        let c4 = make_commit(4, vec![2, 3], "merge1");
        let c5 = make_commit(5, vec![4], "b1");
        let c6 = make_commit(6, vec![4], "f2");
        let c7 = make_commit(7, vec![5, 6], "merge2");
        let commits = vec![c7, c6, c5, c4, c3, c2, c1];

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // With compaction, sequential feature branches should share a column.
        // Without compaction this would need 3 columns (main + 2 features).
        // With compaction, f1 and f2 can share since they don't overlap.
        assert!(
            layout.total_columns <= 2,
            "Sequential branches should compact to ≤2 columns (main + shared feature), got {}",
            layout.total_columns
        );
    }
}
