import type { Edge, GraphLayout } from '$lib/bindings/types';
import {
	columnCenterX,
	nodeCenterY,
	hasArrowGap,
	ARROW_SEGMENT_LENGTH
} from '$lib/graph/graph-math';

export interface EdgeCoords {
	x1: number;
	y1: number;
	x2: number;
	y2: number;
	sameColumn: boolean;
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
}

export function computeEdgeCoords(
	edge: Edge,
	startRow: number,
	rowHeight: number,
	laneWidth: number,
	paddingLeft: number
): EdgeCoords {
	return {
		x1: columnCenterX(edge.from_col, laneWidth, paddingLeft),
		y1: nodeCenterY(edge.from_row, startRow, rowHeight),
		x2: columnCenterX(edge.to_col, laneWidth, paddingLeft),
		y2: nodeCenterY(edge.to_row, startRow, rowHeight),
		sameColumn: edge.from_col === edge.to_col
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

function sampleBezierPoints(
	x1: number,
	y1: number,
	x2: number,
	y2: number,
	n: number
): Array<{ x: number; y: number }> {
	const midX = (x1 + x2) / 2;
	const dy = y2 - y1;
	const cp1x = midX;
	const cp1y = y1 + dy * 0.25;
	const cp2x = midX;
	const cp2y = y2 - dy * 0.25;
	const pts: Array<{ x: number; y: number }> = [];
	for (let i = 0; i <= n; i++) {
		const t = i / n;
		const u = 1 - t;
		pts.push({
			x: u * u * u * x1 + 3 * u * u * t * cp1x + 3 * u * t * t * cp2x + t * t * t * x2,
			y: u * u * u * y1 + 3 * u * u * t * cp1y + 3 * u * t * t * cp2y + t * t * t * y2
		});
	}
	return pts;
}

export function edgeHitTest(mx: number, my: number, coords: EdgeCoords, tolerance = 6): boolean {
	if (coords.sameColumn) {
		return (
			Math.abs(mx - coords.x1) <= tolerance &&
			my >= Math.min(coords.y1, coords.y2) - tolerance &&
			my <= Math.max(coords.y1, coords.y2) + tolerance
		);
	}
	const pts = sampleBezierPoints(coords.x1, coords.y1, coords.x2, coords.y2, 12);
	for (let i = 0; i < pts.length - 1; i++) {
		if (pointToSegmentDist(mx, my, pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y) <= tolerance) {
			return true;
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

		if (hasArrowGap(edge)) {
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
						sameColumn: true
					},
					arrow: 'down',
					fromRow: edge.from_row,
					fromCol: edge.from_col,
					toRow: seg1EndRow,
					toCol: edge.from_col
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
						sameColumn: true
					},
					arrow: 'up',
					fromRow: seg2StartRow,
					fromCol: edge.to_col,
					toRow: edge.to_row,
					toCol: edge.to_col
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
				toCol: edge.to_col
			});
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
	if (coords.sameColumn) {
		ctx.moveTo(coords.x1, coords.y1);
		ctx.lineTo(coords.x2, coords.y2);
	} else {
		const midX = (coords.x1 + coords.x2) / 2;
		const dy = coords.y2 - coords.y1;
		ctx.moveTo(coords.x1, coords.y1);
		ctx.bezierCurveTo(
			midX,
			coords.y1 + dy * 0.25,
			midX,
			coords.y2 - dy * 0.25,
			coords.x2,
			coords.y2
		);
	}
	ctx.stroke();
	ctx.globalAlpha = 1.0;
}

export function drawEdgeEndpoints(
	ctx: CanvasRenderingContext2D,
	coords: EdgeCoords,
	color: string,
	nodeRadius: number,
	arrow: 'down' | 'up' | null = null
) {
	ctx.globalAlpha = 0.8;
	ctx.strokeStyle = color;
	ctx.lineWidth = 1.5;
	ctx.setLineDash([]);
	if (arrow === null || arrow === 'down') {
		ctx.beginPath();
		ctx.arc(coords.x1, coords.y1, nodeRadius + 3, 0, Math.PI * 2);
		ctx.stroke();
	}
	if (arrow === null || arrow === 'up') {
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
