import type { GraphLayout, Edge } from '$lib/bindings/types';

interface InternalEdge {
	from_row: number;
	from_col: number;
	to_row: number;
	to_col: number;
	edge_type: 'Straight' | 'Branch' | 'Merge';
	color: { r: number; g: number; b: number; a: number };
	is_dimmed: boolean;
	edge_style: 'Solid' | 'Dashed' | 'Dotted';
	removable: boolean;
	ancestorIndex: number;
	idx: number;
}

export function computeHideMergeLayout(
	layout: GraphLayout | null,
	hideMerges: boolean
): GraphLayout | null {
	if (!hideMerges || !layout) return layout;

	const mergeRows = new Set(
		layout.nodes.filter((n) => n.is_merge && !n.is_stash).map((n) => n.row)
	);
	if (mergeRows.size === 0) return layout;

	const sortedNodes = [...layout.nodes].sort((a, b) => a.row - b.row);
	const rowMap = new Map<number, number>();
	let newRow = 0;
	for (const node of sortedNodes) {
		if (mergeRows.has(node.row)) continue;
		rowMap.set(node.row, newRow);
		newRow++;
	}

	const edges = layout.edges;
	const ancestorCache = new Map<
		number,
		{ rows: Array<{ row: number; col: number }>; seen: Set<number> }
	>();

	const edgesByFrom = new Map<number, Edge[]>();
	for (const e of edges) {
		const list = edgesByFrom.get(e.from_row) ?? [];
		list.push(e);
		edgesByFrom.set(e.from_row, list);
	}

	function collectNonMergeAncestors(
		row: number,
		col: number,
		visited?: Set<number>
	): Array<{ row: number; col: number }> {
		if (!mergeRows.has(row)) return [{ row, col }];
		const cached = ancestorCache.get(row);
		if (cached) return cached.rows;
		visited = visited ?? new Set();
		if (visited.has(row)) return [];
		visited.add(row);
		const result: Array<{ row: number; col: number }> = [];
		const seen = new Set<number>();
		const outEdges = edgesByFrom.get(row) ?? [];
		for (const e of outEdges) {
			if (e.edge_type === 'Merge') continue;
			for (const a of collectNonMergeAncestors(e.to_row, e.to_col, visited)) {
				if (!seen.has(a.row)) {
					seen.add(a.row);
					result.push(a);
				}
			}
		}
		for (const e of outEdges) {
			if (e.edge_type !== 'Merge') continue;
			for (const a of collectNonMergeAncestors(e.to_row, e.to_col, visited)) {
				if (!seen.has(a.row)) {
					seen.add(a.row);
					result.push(a);
				}
			}
		}
		result.sort((a, b) => a.row - b.row);
		ancestorCache.set(row, { rows: result, seen });
		return result;
	}

	const showIncoming = new Map<number, number>();
	const showOutgoing = new Map<number, number>();
	for (const n of sortedNodes) {
		if (mergeRows.has(n.row)) continue;
		showIncoming.set(n.row, 0);
		showOutgoing.set(n.row, 0);
	}
	for (const e of edges) {
		if (!mergeRows.has(e.to_row)) {
			showIncoming.set(e.to_row, (showIncoming.get(e.to_row) ?? 0) + 1);
		}
		if (!mergeRows.has(e.from_row)) {
			showOutgoing.set(e.from_row, (showOutgoing.get(e.from_row) ?? 0) + 1);
		}
	}

	const mergeChildren = new Map<number, number[]>();
	for (const e of edges) {
		if (mergeRows.has(e.from_row) || !mergeRows.has(e.to_row)) continue;
		const list = mergeChildren.get(e.to_row) ?? [];
		if (!list.includes(e.from_row)) list.push(e.from_row);
		mergeChildren.set(e.to_row, list);
	}

	const nodeColumns = new Map<number, number>();
	for (const n of sortedNodes) {
		nodeColumns.set(n.row, n.column);
	}

	const iEdges: InternalEdge[] = [];
	const edgeSet = new Set<string>();
	let nextIdx = 0;

	const nonRemOutCount = new Map<number, number>();
	const nonRemInCount = new Map<number, number>();

	for (const e of edges) {
		if (mergeRows.has(e.from_row) || mergeRows.has(e.to_row)) continue;
		const mappedFrom = rowMap.get(e.from_row)!;
		const mappedTo = rowMap.get(e.to_row)!;
		iEdges.push({
			from_row: mappedFrom,
			from_col: e.from_col,
			to_row: mappedTo,
			to_col: e.to_col,
			edge_type: e.edge_type,
			color: e.color,
			is_dimmed: e.is_dimmed,
			edge_style: e.edge_style,
			removable: false,
			ancestorIndex: -1,
			idx: nextIdx++
		});
		edgeSet.add(`${mappedFrom}:${mappedTo}`);
		nonRemOutCount.set(e.from_row, (nonRemOutCount.get(e.from_row) ?? 0) + 1);
		nonRemInCount.set(e.to_row, (nonRemInCount.get(e.to_row) ?? 0) + 1);
	}

	const processedMerges = new Set<number>();
	for (const e of edges) {
		if (mergeRows.has(e.from_row) || !mergeRows.has(e.to_row)) continue;
		if (processedMerges.has(e.to_row)) continue;
		processedMerges.add(e.to_row);

		const mergeRow = e.to_row;
		const children = mergeChildren.get(mergeRow) ?? [];
		if (children.length === 0) continue;
		const ancestors = collectNonMergeAncestors(mergeRow, e.to_col);
		if (ancestors.length === 0) continue;

		for (const childRow of children) {
			const childCol = nodeColumns.get(childRow) ?? 0;
			const mappedFrom = rowMap.get(childRow)!;
			for (let ai = 0; ai < ancestors.length; ai++) {
				const a = ancestors[ai];
				const mappedTo = rowMap.get(a.row);
				if (mappedTo === undefined) continue;
				const key = `${mappedFrom}:${mappedTo}`;
				if (edgeSet.has(key)) continue;
				edgeSet.add(key);
				const childNeeds = (nonRemOutCount.get(childRow) ?? 0) < (showOutgoing.get(childRow) ?? 0);
				const ancestorHasRoom = (nonRemInCount.get(a.row) ?? 0) < (showIncoming.get(a.row) ?? 0);
				const makeNonRemovable = childNeeds && ancestorHasRoom;
				if (makeNonRemovable) {
					nonRemOutCount.set(childRow, (nonRemOutCount.get(childRow) ?? 0) + 1);
					nonRemInCount.set(a.row, (nonRemInCount.get(a.row) ?? 0) + 1);
				}
				iEdges.push({
					from_row: mappedFrom,
					from_col: childCol,
					to_row: mappedTo,
					to_col: a.col,
					edge_type: childCol === a.col ? 'Straight' : 'Branch',
					color: e.color,
					is_dimmed: false,
					edge_style: 'Solid',
					removable: !makeNonRemovable,
					ancestorIndex: ai,
					idx: nextIdx++
				});
			}
		}
	}

	const hideIncoming = new Map<number, number[]>();
	const hideOutgoing = new Map<number, number[]>();
	for (let i = 0; i < iEdges.length; i++) {
		const e = iEdges[i];
		let list = hideIncoming.get(e.to_row);
		if (!list) {
			list = [];
			hideIncoming.set(e.to_row, list);
		}
		list.push(i);
		list = hideOutgoing.get(e.from_row);
		if (!list) {
			list = [];
			hideOutgoing.set(e.from_row, list);
		}
		list.push(i);
	}

	const removed = new Set<number>();

	function pruneEdge(idx: number) {
		if (removed.has(idx)) return;
		removed.add(idx);
		const e = iEdges[idx];
		const inList = hideIncoming.get(e.to_row);
		if (inList) {
			const pos = inList.indexOf(idx);
			if (pos >= 0) inList.splice(pos, 1);
		}
		const outList = hideOutgoing.get(e.from_row);
		if (outList) {
			const pos = outList.indexOf(idx);
			if (pos >= 0) outList.splice(pos, 1);
		}
	}

	function getRemovableIncoming(row: number): number[] {
		return (hideIncoming.get(row) ?? []).filter((i) => iEdges[i].removable);
	}

	function getRemovableOutgoing(row: number): number[] {
		return (hideOutgoing.get(row) ?? []).filter((i) => iEdges[i].removable);
	}

	for (const [origRow, mappedRow] of rowMap) {
		if ((showIncoming.get(origRow) ?? 0) === 0) {
			for (const ri of getRemovableIncoming(mappedRow)) {
				pruneEdge(ri);
			}
		}
	}

	for (const [origRow, mappedRow] of rowMap) {
		const limit = showOutgoing.get(origRow) ?? 0;
		const current = hideOutgoing.get(mappedRow)?.length ?? 0;
		if (current <= limit) continue;

		let hasOnlyMergeParents = true;
		for (const e of edges) {
			if (e.from_row !== origRow) continue;
			if (!mergeRows.has(e.to_row)) {
				hasOnlyMergeParents = false;
				break;
			}
		}

		const removable = getRemovableOutgoing(mappedRow);
		if (hasOnlyMergeParents && removable.length > 0) {
			let needsRelaxation = false;
			for (const ri of removable) {
				const targetIncoming = hideIncoming.get(iEdges[ri].to_row);
				if (targetIncoming && targetIncoming.every((idx) => iEdges[idx].removable)) {
					needsRelaxation = true;
					break;
				}
			}
			if (needsRelaxation) continue;
		}

		const toRemove = current - limit;
		removable.sort((ai, bi) => {
			const a = iEdges[ai];
			const b = iEdges[bi];
			const aInc = hideIncoming.get(a.to_row)?.length ?? 0;
			const bInc = hideIncoming.get(b.to_row)?.length ?? 0;
			if (aInc !== bInc) return bInc - aInc;
			const aMatch = a.from_col === a.to_col ? 0 : 1;
			const bMatch = b.from_col === b.to_col ? 0 : 1;
			if (aMatch !== bMatch) return bMatch - aMatch;
			return b.ancestorIndex - a.ancestorIndex;
		});
		for (let i = 0; i < toRemove && i < removable.length; i++) {
			pruneEdge(removable[i]);
		}
	}

	for (const [origRow, mappedRow] of rowMap) {
		const limit = showIncoming.get(origRow) ?? 0;
		const current = hideIncoming.get(mappedRow)?.length ?? 0;
		if (current <= limit) continue;
		const removable = getRemovableIncoming(mappedRow);
		const toRemove = current - limit;
		for (let i = 0; i < toRemove && i < removable.length; i++) {
			pruneEdge(removable[i]);
		}
	}

	const finalEdges: Edge[] = [];
	for (let i = 0; i < iEdges.length; i++) {
		if (removed.has(i)) continue;
		const ie = iEdges[i];
		finalEdges.push({
			from_row: ie.from_row,
			from_col: ie.from_col,
			to_row: ie.to_row,
			to_col: ie.to_col,
			edge_type: ie.edge_type,
			color: ie.color,
			is_dimmed: ie.is_dimmed,
			edge_style: ie.edge_style
		});
	}

	return {
		...layout,
		nodes: sortedNodes
			.filter((n) => !mergeRows.has(n.row))
			.map((n) => ({ ...n, row: rowMap.get(n.row)! })),
		edges: finalEdges,
		stash_markers: layout.stash_markers
			.filter((s) => !mergeRows.has(s.row))
			.map((s) => ({ ...s, row: rowMap.get(s.row)! })),
		total_rows: rowMap.size
	};
}
