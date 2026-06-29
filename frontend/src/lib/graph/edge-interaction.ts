import type { Edge, GraphLayout } from '$lib/bindings/types';
import {
	columnCenterX,
	nodeCenterY,
	hasArrowGap,
	isCrossColumn,
	ARROW_SEGMENT_LENGTH
} from '$lib/graph/graph-math';

const CHAMFER_FRAC = 0.5;

export interface EdgeCoords {
	x1: number;
	y1: number;
	x2: number;
	y2: number;
	sameColumn: boolean;
	/** Pixel coordinates of waypoints (unscaled) */
	wpPx: { x: number; y: number }[];
}

export interface VisibleEdgeSegment {
	edge: Edge;
	idx: number;
	coords: EdgeCoords;
	arrow: 'down' | 'up' | null;
	fromRow: number;
	fromCol: number;
	toRow: number;
	toCol: number;
	isEdgeStart?: boolean;
	isEdgeEnd?: boolean;
}

export function computeEdgeCoords(
	edge: Edge,
	startRow: number,
	rowHeight: number,
	laneWidth: number,
	paddingLeft: number
): EdgeCoords {
	const wpPx = (edge.waypoints ?? []).map(([r, c]) => ({
		x: columnCenterX(c, laneWidth, paddingLeft),
		y: nodeCenterY(r, startRow, rowHeight)
	}));
	return {
		x1: columnCenterX(edge.from_col, laneWidth, paddingLeft),
		y1: nodeCenterY(edge.from_row, startRow, rowHeight),
		x2: columnCenterX(edge.to_col, laneWidth, paddingLeft),
		y2: nodeCenterY(edge.to_row, startRow, rowHeight),
		sameColumn: edge.from_col === edge.to_col,
		wpPx
	};
}

function pointToSegmentDist(
	px: number,
	my: number,
	ax: number,
	ay: number,
	bx: number,
	by: number
): number {
	const dx = bx - ax;
	const dy = by - ay;
	const lenSq = dx * dx + dy * dy;
	if (lenSq === 0) return Math.hypot(px - ax, my - ay);
	let t = ((px - ax) * dx + (my - ay) * dy) / lenSq;
	t = Math.max(0, Math.min(1, t));
	return Math.hypot(px - (ax + t * dx), my - (ay + t * dy));
}

export function edgeHitTest(mx: number, my: number, coords: EdgeCoords, tolerance = 6): boolean {
	// Build the list of points to test (from → waypoints → to)
	const points: Array<{ x: number; y: number }> = [
		{ x: coords.x1, y: coords.y1 },
		...coords.wpPx,
		{ x: coords.x2, y: coords.y2 }
	];

	// Test each segment
	for (let i = 0; i < points.length - 1; i++) {
		const a = points[i];
		const b = points[i + 1];
		if (a.x === b.x) {
			// Vertical segment
			if (
				Math.abs(mx - a.x) <= tolerance &&
				my >= Math.min(a.y, b.y) - tolerance &&
				my <= Math.max(a.y, b.y) + tolerance
			) {
				return true;
			}
		} else if (a.y === b.y) {
			// Horizontal segment
			if (
				Math.abs(my - a.y) <= tolerance &&
				mx >= Math.min(a.x, b.x) - tolerance &&
				mx <= Math.max(a.x, b.x) + tolerance
			) {
				return true;
			}
		} else {
			if (pointToSegmentDist(mx, my, a.x, a.y, b.x, b.y) <= tolerance) {
				return true;
			}
		}
	}
	return false;
}

export function edgeFarOid(
	edge: Edge,
	layout: GraphLayout,
	selectedOid: string | null,
	arrow: 'down' | 'up' | null = null
): string | null {
	const fromNode = layout.nodes.find((n) => n.row === edge.from_row);
	const toNode = layout.nodes.find((n) => n.row === edge.to_row);
	if (!fromNode || !toNode) return null;
	if (arrow === 'down') return toNode.oid;
	if (arrow === 'up') return fromNode.oid;
	if (!selectedOid) return toNode.oid;
	if (selectedOid === fromNode.oid) return toNode.oid;
	if (selectedOid === toNode.oid) return fromNode.oid;
	const selectedNode = layout.nodes.find((n) => n.oid === selectedOid);
	if (!selectedNode) return toNode.oid;
	const distFrom = Math.abs(selectedNode.row - edge.from_row);
	const distTo = Math.abs(selectedNode.row - edge.to_row);
	return distFrom > distTo ? fromNode.oid : toNode.oid;
}

function pushSegPx(
	result: VisibleEdgeSegment[],
	edge: Edge,
	idx: number,
	x1: number,
	y1: number,
	x2: number,
	y2: number,
	fromRow: number,
	fromCol: number,
	toRow: number,
	toCol: number,
	isEdgeStart: boolean,
	isEdgeEnd: boolean
) {
	result.push({
		edge,
		idx,
		coords: { x1, y1, x2, y2, sameColumn: fromCol === toCol, wpPx: [] },
		arrow: null,
		fromRow,
		fromCol,
		toRow,
		toCol,
		isEdgeStart,
		isEdgeEnd
	});
}

export function computeVisibleEdgeCoords(
	layout: GraphLayout,
	startRow: number,
	endRow: number,
	rowHeight: number,
	laneWidth: number,
	paddingLeft: number,
	arrowGapThreshold: number
): VisibleEdgeSegment[] {
	const result: VisibleEdgeSegment[] = [];
	for (let i = 0; i < layout.edges.length; i++) {
		const edge = layout.edges[i];
		const minRow = Math.min(edge.from_row, edge.to_row);
		const maxRow = Math.max(edge.from_row, edge.to_row);
		if (minRow > endRow || maxRow < startRow) continue;

		// Multi-segment edges with waypoints: render as single segment,
		// bypassing arrow gap and chamfering logic.
		if (edge.waypoints && edge.waypoints.length > 0) {
			result.push({
				edge,
				idx: i,
				coords: computeEdgeCoords(edge, startRow, rowHeight, laneWidth, paddingLeft),
				arrow: null,
				fromRow: edge.from_row,
				fromCol: edge.from_col,
				toRow: edge.to_row,
				toCol: edge.to_col,
				isEdgeStart: true,
				isEdgeEnd: true
			});
			continue;
		}

		if (!isCrossColumn(edge)) {
			// Same-column edge: existing arrow gap logic
			if (hasArrowGap(edge, arrowGapThreshold)) {
				const dir = edge.to_row > edge.from_row ? 1 : -1;
				const seg1EndRow = edge.from_row + dir * ARROW_SEGMENT_LENGTH;
				const seg1Lo = Math.min(edge.from_row, seg1EndRow);
				const seg1Hi = Math.max(edge.from_row, seg1EndRow);
				if (seg1Lo < endRow && seg1Hi > startRow) {
					const x = columnCenterX(edge.from_col, laneWidth, paddingLeft);
					result.push({
						edge,
						idx: i,
						coords: {
							x1: x,
							y1: nodeCenterY(edge.from_row, startRow, rowHeight),
							x2: x,
							y2: nodeCenterY(seg1EndRow, startRow, rowHeight),
							sameColumn: true,
							wpPx: []
						},
						arrow: 'down',
						fromRow: edge.from_row,
						fromCol: edge.from_col,
						toRow: seg1EndRow,
						toCol: edge.from_col,
						isEdgeStart: true,
						isEdgeEnd: false
					});
				}
				const seg2StartRow = edge.to_row - dir * ARROW_SEGMENT_LENGTH;
				const seg2Lo = Math.min(seg2StartRow, edge.to_row);
				const seg2Hi = Math.max(seg2StartRow, edge.to_row);
				if (seg2Lo < endRow && seg2Hi > startRow) {
					const x = columnCenterX(edge.to_col, laneWidth, paddingLeft);
					result.push({
						edge,
						idx: i,
						coords: {
							x1: x,
							y1: nodeCenterY(seg2StartRow, startRow, rowHeight),
							x2: x,
							y2: nodeCenterY(edge.to_row, startRow, rowHeight),
							sameColumn: true,
							wpPx: []
						},
						arrow: 'up',
						fromRow: seg2StartRow,
						fromCol: edge.to_col,
						toRow: edge.to_row,
						toCol: edge.to_col,
						isEdgeStart: false,
						isEdgeEnd: true
					});
				}
			} else {
				result.push({
					edge,
					idx: i,
					coords: computeEdgeCoords(edge, startRow, rowHeight, laneWidth, paddingLeft),
					arrow: null,
					fromRow: edge.from_row,
					fromCol: edge.from_col,
					toRow: edge.to_row,
					toCol: edge.to_col,
					isEdgeStart: true,
					isEdgeEnd: true
				});
			}
		} else {
			const cX = columnCenterX(edge.from_col, laneWidth, paddingLeft);
			const pX = columnCenterX(edge.to_col, laneWidth, paddingLeft);
			const cY = nodeCenterY(edge.from_row, startRow, rowHeight);
			const pY = nodeCenterY(edge.to_row, startRow, rowHeight);

			if (hasArrowGap(edge, arrowGapThreshold)) {
				// Long cross-column edge: orthogonal routing with arrow gaps
				const dir = edge.to_row > edge.from_row ? 1 : -1;
				const dxSign = edge.to_col > edge.from_col ? 1 : -1;
				const seg1EndRow = edge.from_row + dir * ARROW_SEGMENT_LENGTH;
				const seg2StartRow = edge.to_row - dir * ARROW_SEGMENT_LENGTH;
				const seg1EndY = nodeCenterY(seg1EndRow, startRow, rowHeight);
				const seg2StartY = nodeCenterY(seg2StartRow, startRow, rowHeight);
				const chamfer = laneWidth * CHAMFER_FRAC;

				if (edge.edge_type === 'Branch') {
					// Corner at (child's col, parent-3 row)
					// 1. vertical (child+3 → corner - chamfer) in child's col
					// 2. diagonal chamfer at corner
					// 3. horizontal at parent-3 row to parent's col

					// 1. 'down' at child's column
					pushSegPx(
						result,
						edge,
						i,
						cX,
						cY,
						cX,
						seg1EndY,
						edge.from_row,
						edge.from_col,
						seg1EndRow,
						edge.from_col,
						true,
						false
					);
					result[result.length - 1].arrow = 'down';

					// 2. vertical in child's column: child+3 → parent-3 (minus chamfer)
					pushSegPx(
						result,
						edge,
						i,
						cX,
						seg1EndY,
						cX,
						seg2StartY - dir * chamfer,
						seg1EndRow,
						edge.from_col,
						seg2StartRow,
						edge.from_col,
						false,
						false
					);

					// 3. diagonal chamfer at corner
					pushSegPx(
						result,
						edge,
						i,
						cX,
						seg2StartY - dir * chamfer,
						cX + dxSign * chamfer,
						seg2StartY,
						seg2StartRow,
						edge.from_col,
						seg2StartRow,
						edge.to_col,
						false,
						false
					);

					// 4. horizontal at parent-3 row to parent's column
					pushSegPx(
						result,
						edge,
						i,
						cX + dxSign * chamfer,
						seg2StartY,
						pX,
						seg2StartY,
						seg2StartRow,
						edge.from_col,
						seg2StartRow,
						edge.to_col,
						false,
						false
					);

					// 5. 'up' at parent's column
					pushSegPx(
						result,
						edge,
						i,
						pX,
						seg2StartY,
						pX,
						pY,
						seg2StartRow,
						edge.to_col,
						edge.to_row,
						edge.to_col,
						false,
						true
					);
					result[result.length - 1].arrow = 'up';
				} else {
					// Corner at (parent's col, child+3 row)
					// 1. horizontal at child+3 row toward parent's col
					// 2. diagonal chamfer at corner
					// 3. vertical (corner + chamfer → parent-3) in parent's col

					// 1. 'down' at child's column
					pushSegPx(
						result,
						edge,
						i,
						cX,
						cY,
						cX,
						seg1EndY,
						edge.from_row,
						edge.from_col,
						seg1EndRow,
						edge.from_col,
						true,
						false
					);
					result[result.length - 1].arrow = 'down';

					// 2. horizontal at child+3 row toward parent's column
					pushSegPx(
						result,
						edge,
						i,
						cX,
						seg1EndY,
						pX - dxSign * chamfer,
						seg1EndY,
						seg1EndRow,
						edge.from_col,
						seg1EndRow,
						edge.to_col,
						false,
						false
					);

					// 3. diagonal chamfer at corner
					pushSegPx(
						result,
						edge,
						i,
						pX - dxSign * chamfer,
						seg1EndY,
						pX,
						seg1EndY + dir * chamfer,
						seg1EndRow,
						edge.from_col,
						seg1EndRow,
						edge.to_col,
						false,
						false
					);

					// 4. vertical in parent's column: child+3 + chamfer → parent-3
					pushSegPx(
						result,
						edge,
						i,
						pX,
						seg1EndY + dir * chamfer,
						pX,
						seg2StartY,
						seg1EndRow,
						edge.to_col,
						seg2StartRow,
						edge.to_col,
						false,
						false
					);

					// 5. 'up' at parent's column
					pushSegPx(
						result,
						edge,
						i,
						pX,
						seg2StartY,
						pX,
						pY,
						seg2StartRow,
						edge.to_col,
						edge.to_row,
						edge.to_col,
						false,
						true
					);
					result[result.length - 1].arrow = 'up';
				}
			} else {
				// Short cross-column edge: 3-segment chamfered orthogonal path
				const chamfer = laneWidth * CHAMFER_FRAC;
				const dxSign = edge.to_col > edge.from_col ? 1 : -1;
				const drSign = edge.to_row > edge.from_row ? 1 : -1;

				if (edge.edge_type === 'Branch') {
					// Corner = (cX, pY)  — child's column, parent's row
					pushSegPx(
						result,
						edge,
						i,
						cX,
						cY,
						cX,
						pY - drSign * chamfer,
						edge.from_row,
						edge.from_col,
						edge.to_row,
						edge.from_col,
						true,
						false
					);
					pushSegPx(
						result,
						edge,
						i,
						cX,
						pY - drSign * chamfer,
						cX + dxSign * chamfer,
						pY,
						edge.to_row,
						edge.from_col,
						edge.to_row,
						edge.to_col,
						false,
						false
					);
					pushSegPx(
						result,
						edge,
						i,
						cX + dxSign * chamfer,
						pY,
						pX,
						pY,
						edge.to_row,
						edge.from_col,
						edge.to_row,
						edge.to_col,
						false,
						true
					);
				} else {
					// Corner = (pX, cY)  — parent's column, child's row
					pushSegPx(
						result,
						edge,
						i,
						cX,
						cY,
						pX - dxSign * chamfer,
						cY,
						edge.from_row,
						edge.from_col,
						edge.from_row,
						edge.to_col,
						true,
						false
					);
					pushSegPx(
						result,
						edge,
						i,
						pX - dxSign * chamfer,
						cY,
						pX,
						cY + drSign * chamfer,
						edge.from_row,
						edge.to_col,
						edge.to_row,
						edge.to_col,
						false,
						false
					);
					pushSegPx(
						result,
						edge,
						i,
						pX,
						cY + drSign * chamfer,
						pX,
						pY,
						edge.from_row,
						edge.to_col,
						edge.to_row,
						edge.to_col,
						false,
						true
					);
				}
			}
		}
	}
	return result;
}

export function drawEdgeHighlight(
	ctx: CanvasRenderingContext2D,
	coords: EdgeCoords,
	color: string,
	lineWidth = 3.5
) {
	ctx.beginPath();
	ctx.globalAlpha = 1.0;
	ctx.strokeStyle = color;
	ctx.lineWidth = lineWidth;
	ctx.setLineDash([]);
	ctx.moveTo(coords.x1, coords.y1);
	for (const wp of coords.wpPx) {
		ctx.lineTo(wp.x, wp.y);
	}
	ctx.lineTo(coords.x2, coords.y2);
	ctx.stroke();
	ctx.globalAlpha = 1.0;
}

export function isEdgeEndpoint(seg: VisibleEdgeSegment, which: 'from' | 'to'): boolean {
	if (which === 'from') return seg.isEdgeStart === true;
	return seg.isEdgeEnd === true;
}

export function drawEdgeEndpoints(
	ctx: CanvasRenderingContext2D,
	coords: EdgeCoords,
	color: string,
	nodeRadius: number,
	_arrow: 'down' | 'up' | null = null,
	drawStart = true,
	drawEnd = true
) {
	ctx.globalAlpha = 0.8;
	ctx.strokeStyle = color;
	ctx.lineWidth = 1.5;
	ctx.setLineDash([]);
	if (drawStart) {
		ctx.beginPath();
		ctx.arc(coords.x1, coords.y1, nodeRadius + 3, 0, Math.PI * 2);
		ctx.stroke();
	}
	if (drawEnd) {
		ctx.beginPath();
		ctx.arc(coords.x2, coords.y2, nodeRadius + 3, 0, Math.PI * 2);
		ctx.stroke();
	}
	ctx.globalAlpha = 1.0;
}

export function drawArrowHead(
	ctx: CanvasRenderingContext2D,
	x: number,
	y: number,
	color: string,
	direction: 'up' | 'down',
	alpha = 1.0
) {
	const s = 4;
	ctx.beginPath();
	ctx.globalAlpha = alpha;
	ctx.fillStyle = color;
	if (direction === 'down') {
		ctx.moveTo(x, y + s);
		ctx.lineTo(x - s * 0.7, y - s * 0.4);
		ctx.lineTo(x + s * 0.7, y - s * 0.4);
	} else {
		ctx.moveTo(x, y - s);
		ctx.lineTo(x - s * 0.7, y + s * 0.4);
		ctx.lineTo(x + s * 0.7, y + s * 0.4);
	}
	ctx.closePath();
	ctx.fill();
	ctx.globalAlpha = 1.0;
}
