<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { GraphLayout, NodePosition, Edge, CommitInfo } from '$lib/bindings/types';
	import { updateGraphDrawTime } from '$lib/stores/debug';
	import {
		colorToCSS,
		columnCenterX,
		nodeCenterY,
		isEdgeVisible,
		SELECTED_COLOR,
		COMPARISON_COLOR,
		STASH_COLOR
	} from '$lib/graph/graph-math';
	import { computeVisibleEdgeCoords, edgeHitTest, edgeFarOid } from '$lib/graph/edge-interaction';

	const PADDING_LEFT = 12;
	const EDGE_HIT_TOLERANCE = 6;

	interface Props {
		layout: GraphLayout;
		commits: CommitInfo[];
		rowHeight?: number;
		laneWidth?: number;
		nodeRadius?: number;
		visibleStart: number;
		visibleEnd: number;
		selectedOid?: string | null;
		comparisonOid?: string | null;
		onSelect?: (_oid: string, _ctrlKey: boolean) => void;
		onEdgeNavigate?: (_oid: string) => void;
	}

	let {
		layout,
		commits,
		rowHeight = 28,
		laneWidth = 24,
		nodeRadius = 4,
		visibleStart,
		visibleEnd,
		selectedOid,
		comparisonOid = null,
		onSelect,
		onEdgeNavigate
	}: Props = $props();

	let canvas: HTMLCanvasElement;
	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);
	let scale = $state(1.0);
	let prevCanvasW = 0;
	let prevCanvasH = 0;
	let hoveredEdgeIdx = $state<number | null>(null);
	let selectedEdgeIdx = $state<number | null>(null);

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	let visibleEdgeData = $derived(
		computeVisibleEdgeCoords(layout, visibleStart, visibleEnd, rowHeight, laneWidth, PADDING_LEFT)
	);

	function draw(l: GraphLayout) {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		const drawStart = performance.now();

		const sc = scale;
		const sLaneWidth = laneWidth * sc;
		const sNodeRadius = nodeRadius * sc;
		const sPadding = PADDING_LEFT * sc;

		const height = (visibleEnd - visibleStart) * rowHeight;
		const width = l.total_columns * sLaneWidth + sPadding * 2;
		if (width <= 0 || height <= 0) return;

		const cssW = `${width}px`;
		const cssH = `${height}px`;
		if (width !== prevCanvasW || height !== prevCanvasH) {
			canvas.width = width * devicePixelRatio;
			canvas.height = height * devicePixelRatio;
			canvas.style.width = cssW;
			canvas.style.height = cssH;
			prevCanvasW = width;
			prevCanvasH = height;
		}
		ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
		ctx.clearRect(0, 0, width, height);

		const startRow = visibleStart;
		const endRow = visibleEnd;

		for (let edgeIdx = 0; edgeIdx < l.edges.length; edgeIdx++) {
			const edge = l.edges[edgeIdx];
			if (!isEdgeVisible(edge, startRow, endRow)) continue;
			const isSelected = edgeIdx === selectedEdgeIdx;
			const isHovered = edgeIdx === hoveredEdgeIdx && !isSelected;
			drawEdge(ctx, edge, sLaneWidth, sPadding, startRow, rowHeight, isHovered, isSelected);
		}
		const stashIdxMap = new Map(l.stash_markers.map((s) => [s.stash_oid, s.stash_index]));
		for (const node of l.nodes) {
			if (node.row < startRow || node.row > endRow) continue;
			drawNode(ctx, node, sLaneWidth, sNodeRadius, sPadding, startRow, rowHeight, sc, stashIdxMap);
		}
		updateGraphDrawTime(performance.now() - drawStart);
	}

	$effect(() => {
		void visibleStart;
		void visibleEnd;
		void hoveredEdgeIdx;
		void selectedEdgeIdx;
		draw(layout);
	});

	function handleEdgeClick(mx: number, my: number): boolean {
		for (const { edge, idx, coords } of visibleEdgeData) {
			if (edgeHitTest(mx, my, coords, EDGE_HIT_TOLERANCE)) {
				if (selectedEdgeIdx === idx) {
					const farOid = edgeFarOid(edge, layout, selectedOid ?? null);
					if (farOid) {
						onEdgeNavigate?.(farOid);
						onSelect?.(farOid, false);
					}
					selectedEdgeIdx = null;
				} else {
					selectedEdgeIdx = idx;
				}
				return true;
			}
		}
		return false;
	}

	function handleClick(e: MouseEvent) {
		const rect = canvas.getBoundingClientRect();
		const mx = e.clientX - rect.left;
		const my = e.clientY - rect.top;
		const sc = scale;
		const sLaneWidth = laneWidth * sc;
		const sNodeRadius = nodeRadius * sc;
		const sPadding = PADDING_LEFT * sc;
		const hitRadius = sNodeRadius + 4 * sc;

		for (const node of layout.nodes) {
			if (node.row < visibleStart || node.row > visibleEnd) continue;
			const nx = columnCenterX(node.column, sLaneWidth, sPadding);
			const ny = nodeCenterY(node.row, visibleStart, rowHeight);
			if (Math.abs(mx - nx) < hitRadius && Math.abs(my - ny) < hitRadius) {
				selectedEdgeIdx = null;
				onSelect?.(node.oid, e.ctrlKey || e.metaKey);
				return;
			}
		}

		if (!handleEdgeClick(mx, my)) {
			selectedEdgeIdx = null;
		}
	}

	function handleMouseMove(e: MouseEvent) {
		const rect = canvas.getBoundingClientRect();
		const sc = scale;
		const sLaneWidth = laneWidth * sc;
		const sNodeRadius = nodeRadius * sc;
		const sPadding = PADDING_LEFT * sc;
		const x = e.clientX - rect.left;
		const y = e.clientY - rect.top;
		const row = Math.floor(y / rowHeight) + visibleStart;
		const hitRadius = sNodeRadius + 4 * sc;

		for (const node of layout.nodes) {
			if (node.row === row) {
				const nx = columnCenterX(node.column, sLaneWidth, sPadding);
				const ny = nodeCenterY(node.row, visibleStart, rowHeight);
				if (Math.abs(x - nx) < hitRadius && Math.abs(y - ny) < hitRadius) {
					if (node.is_stash) {
						const stash = layout.stash_markers.find((s) => s.stash_oid === node.oid);
						tooltip = {
							x: e.clientX - rect.left + 12,
							y: e.clientY - rect.top - 8,
							text: stash?.message ?? node.oid.substring(0, 7) + ' stash'
						};
					} else {
						const ci = commitMap.get(node.oid);
						if (ci) {
							tooltip = {
								x: e.clientX - rect.left + 12,
								y: e.clientY - rect.top - 8,
								text: `${ci.short_oid} ${ci.summary}`
							};
						}
					}
					hoveredEdgeIdx = null;
					return;
				}
			}
		}

		let newHovered: number | null = null;
		for (const { idx, coords } of visibleEdgeData) {
			if (edgeHitTest(x, y, coords, EDGE_HIT_TOLERANCE)) {
				newHovered = idx;
				break;
			}
		}
		hoveredEdgeIdx = newHovered;
		tooltip = null;
	}

	function handleMouseLeave() {
		tooltip = null;
		hoveredEdgeIdx = null;
	}

	function handleWheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		const delta = e.deltaY > 0 ? -0.1 : 0.1;
		scale = Math.max(0.5, Math.min(2.0, scale + delta));
	}

	function drawNode(
		ctx: CanvasRenderingContext2D,
		node: NodePosition,
		sLaneWidth: number,
		sNodeRadius: number,
		sPadding: number,
		startRow: number,
		rh: number,
		sc: number,
		stashIdxMap: Map<string, number>
	) {
		const x = columnCenterX(node.column, sLaneWidth, sPadding);
		const y = nodeCenterY(node.row, startRow, rh);

		if (node.oid === selectedOid) {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius + 3, 0, Math.PI * 2);
			ctx.strokeStyle = SELECTED_COLOR;
			ctx.lineWidth = 2;
			ctx.stroke();
		} else if (node.oid === comparisonOid) {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius + 3, 0, Math.PI * 2);
			ctx.strokeStyle = COMPARISON_COLOR;
			ctx.lineWidth = 2;
			ctx.setLineDash([3, 3]);
			ctx.stroke();
			ctx.setLineDash([]);
		}

		ctx.globalAlpha = node.is_dimmed ? 0.35 : 1.0;
		if (node.is_stash) {
			const s = sNodeRadius * 0.7;
			ctx.beginPath();
			ctx.moveTo(x, y - s);
			ctx.lineTo(x + s, y);
			ctx.lineTo(x, y + s);
			ctx.lineTo(x - s, y);
			ctx.closePath();
			ctx.fillStyle = STASH_COLOR;
		} else {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius, 0, Math.PI * 2);
			ctx.fillStyle = colorToCSS(node.color);
		}
		ctx.fill();
		if (node.is_stash) {
			const idx = stashIdxMap.get(node.oid);
			if (idx !== undefined) {
				ctx.font = `${10 * sc}px monospace`;
				ctx.fillStyle = STASH_COLOR;
				ctx.fillText(`S${idx}`, x + sNodeRadius + 2 * sc, y + 3 * sc);
			}
		}
		ctx.globalAlpha = 1.0;
	}

	function drawEdge(
		ctx: CanvasRenderingContext2D,
		edge: Edge,
		sLaneWidth: number,
		sPadding: number,
		startRow: number,
		rh: number,
		isHovered: boolean,
		isSelected: boolean
	) {
		const x1 = columnCenterX(edge.from_col, sLaneWidth, sPadding);
		const y1 = nodeCenterY(edge.from_row, startRow, rh);
		const x2 = columnCenterX(edge.to_col, sLaneWidth, sPadding);
		const y2 = nodeCenterY(edge.to_row, startRow, rh);

		ctx.beginPath();
		ctx.globalAlpha = edge.is_dimmed ? 0.35 : isSelected ? 1.0 : isHovered ? 0.9 : 0.8;
		ctx.strokeStyle = colorToCSS(edge.color);
		ctx.lineWidth = isSelected ? 3.5 : isHovered ? 2.5 : 1.5;

		if (edge.edge_style === 'Dashed') {
			ctx.setLineDash([6, 3]);
		} else if (edge.edge_style === 'Dotted') {
			ctx.setLineDash([2, 3]);
		} else {
			ctx.setLineDash([]);
		}

		if (edge.from_col === edge.to_col) {
			ctx.moveTo(x1, y1);
			ctx.lineTo(x2, y2);
		} else {
			const midY = (y1 + y2) / 2;
			ctx.moveTo(x1, y1);
			ctx.bezierCurveTo(x1, midY, x2, midY, x2, y2);
		}

		ctx.stroke();
		ctx.setLineDash([]);
		ctx.globalAlpha = 1.0;
	}
</script>

<canvas
	bind:this={canvas}
	class="block cursor-pointer"
	aria-label={$t('commit_graph.aria')}
	onclick={handleClick}
	onmousemove={handleMouseMove}
	onmouseleave={handleMouseLeave}
	onwheel={handleWheel}
></canvas>

{#if tooltip}
	<div
		class="pointer-events-none absolute z-50 max-w-[250px] rounded bg-gray-800 px-2 py-1 text-xs text-gray-200 shadow-lg border border-gray-700"
		style="left: {tooltip.x}px; top: {tooltip.y}px;"
	>
		{tooltip.text}
	</div>
{/if}
