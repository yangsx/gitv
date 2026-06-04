<script lang="ts">
	import type { GraphLayout, Color, NodePosition, Edge, StashMarker } from '$lib/bindings/types';

	interface Props {
		layout: GraphLayout;
		rowHeight?: number;
		laneWidth?: number;
		nodeRadius?: number;
		visibleStart: number;
		visibleEnd: number;
	}

	let {
		layout,
		rowHeight = 28,
		laneWidth = 24,
		nodeRadius = 4,
		visibleStart,
		visibleEnd
	}: Props = $props();

	let canvas: HTMLCanvasElement;
	let offscreen: HTMLCanvasElement | null = null;
	let cachedLayoutKey = '';

	const PADDING_LEFT = 12;

	function colorToCss(c: Color): string {
		return `rgba(${c.r},${c.g},${c.b},${(c.a / 255).toFixed(2)})`;
	}

	function layoutKey(l: GraphLayout): string {
		return `${l.total_rows}:${l.total_columns}:${l.nodes.length}:${l.edges.length}:${l.stash_markers.length}`;
	}

	function renderToOffscreen(l: GraphLayout) {
		const height = l.total_rows * rowHeight;
		const width = l.total_columns * laneWidth + PADDING_LEFT * 2;
		if (height <= 0 || width <= 0) return;

		const oc = document.createElement('canvas');
		oc.width = width * devicePixelRatio;
		oc.height = height * devicePixelRatio;
		const ctx = oc.getContext('2d');
		if (!ctx) return;

		ctx.scale(devicePixelRatio, devicePixelRatio);
		oc.style.width = `${width}px`;
		oc.style.height = `${height}px`;

		for (const edge of l.edges) {
			drawEdge(ctx, edge, 0);
		}
		for (const node of l.nodes) {
			drawNode(ctx, node, 0);
		}
		for (const stash of l.stash_markers) {
			drawStashMarker(ctx, stash, 0);
		}

		offscreen = oc;
	}

	$effect(() => {
		if (!canvas || !layout) return;
		const key = layoutKey(layout);
		if (key !== cachedLayoutKey) {
			cachedLayoutKey = key;
			renderToOffscreen(layout);
		}
		blitVisible(layout);
	});

	function blitVisible(l: GraphLayout) {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;

		const height = (visibleEnd - visibleStart) * rowHeight;
		const width = l.total_columns * laneWidth + PADDING_LEFT * 2;

		canvas.width = width * devicePixelRatio;
		canvas.height = height * devicePixelRatio;
		canvas.style.width = `${width}px`;
		canvas.style.height = `${height}px`;
		ctx.scale(devicePixelRatio, devicePixelRatio);
		ctx.clearRect(0, 0, width, height);

		if (!offscreen) {
			drawGraph(ctx, l);
			return;
		}

		const sy = visibleStart * rowHeight * devicePixelRatio;
		const sh = height * devicePixelRatio;
		ctx.save();
		ctx.setTransform(1, 0, 0, 1, 0, 0);
		ctx.drawImage(offscreen, 0, sy, offscreen.width, sh, 0, 0, offscreen.width, sh);
		ctx.restore();
	}

	function drawGraph(ctx: CanvasRenderingContext2D, l: GraphLayout) {
		const startRow = visibleStart;
		const endRow = visibleEnd;

		for (const edge of l.edges) {
			if (edge.from_row < startRow && edge.to_row < startRow) continue;
			if (edge.from_row > endRow && edge.to_row > endRow) continue;
			drawEdge(ctx, edge, startRow);
		}
		for (const node of l.nodes) {
			if (node.row < startRow || node.row > endRow) continue;
			drawNode(ctx, node, startRow);
		}
		for (const stash of l.stash_markers) {
			if (stash.row < startRow || stash.row > endRow) continue;
			drawStashMarker(ctx, stash, startRow);
		}
	}

	function rowY(row: number, startRow: number): number {
		return (row - startRow) * rowHeight + rowHeight / 2;
	}

	function colX(col: number): number {
		return PADDING_LEFT + col * laneWidth + laneWidth / 2;
	}

	function drawNode(ctx: CanvasRenderingContext2D, node: NodePosition, startRow: number) {
		const x = colX(node.column);
		const y = rowY(node.row, startRow);

		ctx.globalAlpha = node.is_dimmed ? 0.35 : 1.0;
		ctx.beginPath();
		ctx.arc(x, y, nodeRadius, 0, Math.PI * 2);
		ctx.fillStyle = colorToCss(node.color);
		ctx.fill();
		ctx.globalAlpha = 1.0;
	}

	function drawEdge(ctx: CanvasRenderingContext2D, edge: Edge, startRow: number) {
		const x1 = colX(edge.from_col);
		const y1 = rowY(edge.from_row, startRow);
		const x2 = colX(edge.to_col);
		const y2 = rowY(edge.to_row, startRow);

		ctx.beginPath();
		ctx.globalAlpha = edge.is_dimmed ? 0.35 : 0.8;
		ctx.strokeStyle = colorToCss(edge.color);
		ctx.lineWidth = 1.5;

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

	function drawStashMarker(ctx: CanvasRenderingContext2D, stash: StashMarker, startRow: number) {
		const x = colX(stash.column) + nodeRadius + 4;
		const y = rowY(stash.row, startRow);

		ctx.font = '10px monospace';
		ctx.fillStyle = '#f59e0b';
		ctx.fillText(`S${stash.stash_index}`, x, y + 3);
	}
</script>

<canvas bind:this={canvas} class="block"></canvas>
