use crate::graph::layout::*;

impl GraphLayout {
    #[must_use]
    pub fn from_visible_range(&self, start_row: usize, end_row: usize) -> GraphViewport {
        let clamped_end = end_row.min(self.total_rows);
        let rows = start_row..clamped_end;

        let nodes: Vec<NodePosition> = self
            .nodes
            .iter()
            .filter(|n| n.row >= start_row && n.row < clamped_end)
            .cloned()
            .collect();

        let stash_markers: Vec<StashMarker> = self
            .stash_markers
            .iter()
            .filter(|m| m.row >= start_row && m.row < clamped_end)
            .cloned()
            .collect();

        let edges: Vec<Edge> = self
            .edges
            .iter()
            .filter(|e| {
                (e.from_row >= start_row && e.from_row < clamped_end)
                    || (e.to_row >= start_row && e.to_row < clamped_end)
                    || (e.from_row < start_row && e.to_row >= clamped_end)
            })
            .cloned()
            .collect();

        GraphViewport {
            rows,
            nodes,
            stash_markers,
            edges,
            total_columns: self.total_columns,
        }
    }
}
