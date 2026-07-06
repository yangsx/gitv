import type { Color, Edge, EdgeType } from '$lib/bindings/types';

// Selection highlight colors (as hex RGB tuples)
export const SELECT_RGB: [number, number, number] = [0x60, 0xa5, 0xfa];
export const COMPARISON_RGB: [number, number, number] = [0xa7, 0x8b, 0xfa];
export const SELECTED_COLOR = '#60a5fa';
export const COMPARISON_COLOR = '#a78bfa';
export const STASH_COLOR = '#f59e0b';

export function columnCenterX(column: number, laneWidth: number, paddingLeft: number): number {
	return column * laneWidth + paddingLeft + laneWidth / 2;
}

export function nodeCenterY(row: number, startRow: number, rowHeight: number): number {
	return (row - startRow) * rowHeight + rowHeight / 2;
}

const colorCache = new Map<string, string>();

export function colorToCSS(c: Color): string {
	const key = `${c.r},${c.g},${c.b},${c.a}`;
	let cached = colorCache.get(key);
	if (!cached) {
		cached = `rgba(${c.r},${c.g},${c.b},${(c.a / 255).toFixed(2)})`;
		colorCache.set(key, cached);
		// Limit cache size to prevent memory leak
		if (colorCache.size > 2000) {
			const firstKey = colorCache.keys().next().value!;
			colorCache.delete(firstKey);
		}
	}
	return cached;
}

export function isEdgeVisible(edge: Edge, startRow: number, endRow: number): boolean {
	const minRow = Math.min(edge.from_row, edge.to_row);
	const maxRow = Math.max(edge.from_row, edge.to_row);
	return minRow <= endRow && maxRow >= startRow;
}

export function isCrossColumn(edge: Edge): boolean {
	return edge.from_col !== edge.to_col;
}

export type { EdgeType };

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
