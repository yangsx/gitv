import type { Edge, GraphLayout } from '$lib/bindings/types';
import { columnCenterX, nodeCenterY, isCrossColumn } from '$lib/graph/graph-math';

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
	paddingLeft: number
): VisibleEdgeSegment[] {
	const result: VisibleEdgeSegment[] = [];
	for (let i = 0; i < layout.edges.length; i++) {
		const edge = layout.edges[i];
		const minRow = Math.min(edge.from_row, edge.to_row);
		const maxRow = Math.max(edge.from_row, edge.to_row);
		if (minRow > endRow || maxRow < startRow) continue;

		// Edges with waypoints: trace thread path through intermediate rows
		if (edge.waypoints && edge.waypoints.length > 0) {
			if (edge.arrow_gap) {
				// Thread was removed from rowidlist (gitk thread lifecycle).
				// Split into two segments with arrowheads at the gap boundaries.
				const [seg1EndRow, seg2StartRow] = edge.arrow_gap;

				// Segment 1: from child → seg1EndRow (arrow at far end)
				const seg1Lo = Math.min(edge.from_row, seg1EndRow);
				const seg1Hi = Math.max(edge.from_row, seg1EndRow);
				if (seg1Lo < endRow && seg1Hi > startRow) {
					const seg1Wps = (edge.waypoints ?? []).filter(
						([r]) => r >= seg1Lo && r <= seg1Hi && r !== edge.from_row
					);
					const seg1EndCol = seg1Wps.length > 0 ? seg1Wps[seg1Wps.length - 1][1] : edge.from_col;
					const wpPx1 = seg1Wps
						.filter(([r]) => r !== seg1EndRow)
						.map(([r, c]) => ({
							x: columnCenterX(c, laneWidth, paddingLeft),
							y: nodeCenterY(r, startRow, rowHeight)
						}));
					result.push({
						edge,
						idx: i,
						coords: {
							x1: columnCenterX(edge.from_col, laneWidth, paddingLeft),
							y1: nodeCenterY(edge.from_row, startRow, rowHeight),
							x2: columnCenterX(seg1EndCol, laneWidth, paddingLeft),
							y2: nodeCenterY(seg1EndRow, startRow, rowHeight),
							sameColumn: edge.from_col === seg1EndCol,
							wpPx: wpPx1
						},
						arrow: 'down',
						fromRow: edge.from_row,
						fromCol: edge.from_col,
						toRow: seg1EndRow,
						toCol: seg1EndCol,
						isEdgeStart: true,
						isEdgeEnd: false
					});
				}

				// Segment 2: from seg2StartRow → parent (arrow at near end)
				const seg2Lo = Math.min(seg2StartRow, edge.to_row);
				const seg2Hi = Math.max(seg2StartRow, edge.to_row);
				if (seg2Lo < endRow && seg2Hi > startRow) {
					const seg2Wps = (edge.waypoints ?? []).filter(
						([r]) => r >= seg2Lo && r <= seg2Hi && r !== edge.to_row
					);
					const seg2StartCol = seg2Wps.length > 0 ? seg2Wps[0][1] : edge.to_col;
					const wpPx2 = seg2Wps
						.filter(([r]) => r !== seg2StartRow)
						.map(([r, c]) => ({
							x: columnCenterX(c, laneWidth, paddingLeft),
							y: nodeCenterY(r, startRow, rowHeight)
						}));
					result.push({
						edge,
						idx: i,
						coords: {
							x1: columnCenterX(seg2StartCol, laneWidth, paddingLeft),
							y1: nodeCenterY(seg2StartRow, startRow, rowHeight),
							x2: columnCenterX(edge.to_col, laneWidth, paddingLeft),
							y2: nodeCenterY(edge.to_row, startRow, rowHeight),
							sameColumn: seg2StartCol === edge.to_col,
							wpPx: wpPx2
						},
						arrow: 'up',
						fromRow: seg2StartRow,
						fromCol: seg2StartCol,
						toRow: edge.to_row,
						toCol: edge.to_col,
						isEdgeStart: false,
						isEdgeEnd: true
					});
				}
			} else {
				// No gap: render as single segment through waypoints
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
			continue;
		}

		if (!isCrossColumn(edge)) {
			// Same-column edge: single continuous segment (no arrow gap).
			// Thread-removal gaps are handled by arrow_gap above.
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
		} else {
			// Cross-column edge: distinguish merge back edges from branch edges.
			// Merge back edges (second+ parent of a merge commit) render gitk-style:
			//   neighboring cols → direct diagonal (trunk-to-branch intuition)
			//   non-neighboring  → horizontal-first chamfer (horizontal at merge row,
			//                        then diagonal, then vertical at parent col)
			// Branch edges render vertical-first chamfer (vertical at child col,
			// avoids overlapping mainline nodes at parent col).
			const cX = columnCenterX(edge.from_col, laneWidth, paddingLeft);
			const pX = columnCenterX(edge.to_col, laneWidth, paddingLeft);
			const cY = nodeCenterY(edge.from_row, startRow, rowHeight);
			const pY = nodeCenterY(edge.to_row, startRow, rowHeight);
			const chamfer = laneWidth * CHAMFER_FRAC;
			const dxSign = edge.to_col > edge.from_col ? 1 : -1;
			const drSign = edge.to_row > edge.from_row ? 1 : -1;

			if (edge.edge_type === 'Merge') {
				// Merge back edge: gitk-style rendering
				const colDiff = Math.abs(edge.to_col - edge.from_col);
				if (colDiff <= 1) {
					// Neighboring columns: direct diagonal
					pushSegPx(
						result,
						edge,
						i,
						cX,
						cY,
						pX,
						pY,
						edge.from_row,
						edge.from_col,
						edge.to_row,
						edge.to_col,
						true,
						true
					);
				} else {
					// Non-neighboring: horizontal-first chamfer (gitk style)
					// Seg 1: horizontal at merge row toward parent column
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
					// Seg 2: diagonal corner
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
					// Seg 3: vertical at parent column
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
			} else {
				// Branch edge: horizontal-first chamfer (gitk parent-link style)
				// Seg 1: horizontal at child row toward parent column
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
				// Seg 2: diagonal corner
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
				// Seg 3: vertical at parent column
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
	arrow: 'down' | 'up' | null = null,
	drawStart = true,
	drawEnd = true
) {
	ctx.globalAlpha = 0.8;
	ctx.strokeStyle = color;
	ctx.lineWidth = 1.5;
	ctx.setLineDash([]);
	if (drawStart && (arrow === null || arrow === 'down')) {
		ctx.beginPath();
		ctx.arc(coords.x1, coords.y1, nodeRadius + 3, 0, Math.PI * 2);
		ctx.stroke();
	}
	if (drawEnd && (arrow === null || arrow === 'up')) {
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
