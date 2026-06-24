import type { Edge, GraphLayout } from '$lib/bindings/types';
import { columnCenterX, nodeCenterY } from '$lib/graph/graph-math';

export interface EdgeCoords {
	x1: number;
	y1: number;
	x2: number;
	y2: number;
	sameColumn: boolean;
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
	selectedOid: string | null
): string | null {
	const fromNode = layout.nodes.find((n) => n.row === edge.from_row);
	const toNode = layout.nodes.find((n) => n.row === edge.to_row);
	if (!fromNode || !toNode) return null;
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
): Array<{ edge: Edge; idx: number; coords: EdgeCoords }> {
	const result: Array<{ edge: Edge; idx: number; coords: EdgeCoords }> = [];
	for (let i = 0; i < layout.edges.length; i++) {
		const edge = layout.edges[i];
		const minRow = Math.min(edge.from_row, edge.to_row);
		const maxRow = Math.max(edge.from_row, edge.to_row);
		if (minRow > endRow || maxRow < startRow) continue;
		result.push({
			edge,
			idx: i,
			coords: computeEdgeCoords(edge, startRow, rowHeight, laneWidth, paddingLeft)
		});
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
	nodeRadius: number
) {
	ctx.beginPath();
	ctx.globalAlpha = 0.8;
	ctx.strokeStyle = color;
	ctx.lineWidth = 1.5;
	ctx.setLineDash([]);
	ctx.arc(coords.x1, coords.y1, nodeRadius + 3, 0, Math.PI * 2);
	ctx.stroke();
	ctx.beginPath();
	ctx.arc(coords.x2, coords.y2, nodeRadius + 3, 0, Math.PI * 2);
	ctx.stroke();
	ctx.globalAlpha = 1.0;
}
