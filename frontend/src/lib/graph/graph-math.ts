import type { Color, NodePosition, Edge, StashMarker } from '$lib/bindings/types';

// Selection highlight colors (as hex RGB tuples)
export const SELECT_RGB: [number, number, number] = [0x60, 0xa5, 0xfa];
export const COMPARISON_RGB: [number, number, number] = [0xa7, 0x8b, 0xfa];
export const SELECTED_COLOR = '#60a5fa';
export const COMPARISON_COLOR = '#a78bfa';

export function columnCenterX(column: number, laneWidth: number, paddingLeft: number): number {
	return column * laneWidth + paddingLeft + laneWidth / 2;
}

export function nodeCenterY(row: number, startRow: number, rowHeight: number): number {
	return (row - startRow) * rowHeight + rowHeight / 2;
}

export function stashX(
	column: number,
	laneWidth: number,
	paddingLeft: number,
	nodeRadius: number,
	scale: number = 1
): number {
	return columnCenterX(column, laneWidth, paddingLeft) + nodeRadius + 4 * scale;
}

export function stashY(row: number, startRow: number, rowHeight: number): number {
	return nodeCenterY(row, startRow, rowHeight);
}

export function colorToCSS(c: Color): string {
	return `rgba(${c.r},${c.g},${c.b},${(c.a / 255).toFixed(2)})`;
}

export function colorToFloatComponents(c: Color): [number, number, number, number] {
	return [c.r, c.g, c.b, c.a];
}

export function isEdgeVisible(edge: Edge, startRow: number, endRow: number): boolean {
	const inRange = (r: number) => r >= startRow && r <= endRow;
	return inRange(edge.from_row) || inRange(edge.to_row);
}

export function nodeHitTest(
	mx: number,
	my: number,
	nx: number,
	ny: number,
	radius: number,
	margin = 3
): boolean {
	return Math.abs(mx - nx) <= radius + margin && Math.abs(my - ny) <= radius + margin;
}

export function filterVisibleNodes(
	nodes: NodePosition[],
	startRow: number,
	endRow: number,
	laneWidth: number,
	paddingLeft: number,
	rowHeight: number
): Array<{ node: NodePosition; x: number; y: number }> {
	const result: Array<{ node: NodePosition; x: number; y: number }> = [];
	for (const n of nodes) {
		if (n.row < startRow || n.row > endRow) continue;
		result.push({
			node: n,
			x: columnCenterX(n.column, laneWidth, paddingLeft),
			y: nodeCenterY(n.row, startRow, rowHeight)
		});
	}
	return result;
}

export function filterVisibleEdges(edges: Edge[], startRow: number, endRow: number): Edge[] {
	return edges.filter((e) => isEdgeVisible(e, startRow, endRow));
}

export function filterVisibleStashes(
	stashes: StashMarker[],
	startRow: number,
	endRow: number,
	laneWidth: number,
	paddingLeft: number,
	nodeRadius: number,
	rowHeight: number
): Array<{ stash: StashMarker; x: number; y: number }> {
	const result: Array<{ stash: StashMarker; x: number; y: number }> = [];
	for (const s of stashes) {
		if (s.row < startRow || s.row > endRow) continue;
		result.push({
			stash: s,
			x: stashX(s.column, laneWidth, paddingLeft, nodeRadius),
			y: stashY(s.row, startRow, rowHeight)
		});
	}
	return result;
}
