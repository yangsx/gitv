<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type {
		GraphLayout,
		RenderGraphInput,
		RenderEdge,
		CommitInfo,
		NodePosition
	} from '$lib/bindings/types';
	import {
		columnCenterX,
		nodeCenterY,
		nodeHitTest,
		colorToCSS,
		isCrossColumn,
		SELECTED_COLOR,
		COMPARISON_COLOR,
		STASH_COLOR
	} from '$lib/graph/graph-math';
	import {
		edgeHitTest,
		edgeFarOid,
		drawEdgeHighlight,
		drawEdgeEndpoints,
		drawArrowHead,
		computeVisibleEdgeCoords,
		type VisibleEdgeSegment
	} from '$lib/graph/edge-interaction';
	import {
		GRAPH_PADDING_LEFT as PADDING_LEFT,
		GRAPH_EDGE_HIT_TOLERANCE as EDGE_HIT_TOLERANCE
	} from '$lib/constants';
	import { arrowGapThreshold } from '$lib/stores/repository';

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
		hScrollLeft?: number;
		visibleWidth?: number;
	}

	let {
		layout,
		commits,
		rowHeight = 28,
		laneWidth = 24,
		nodeRadius = 4,
		visibleStart,
		visibleEnd,
		selectedOid = null,
		comparisonOid = null,
		onSelect,
		onEdgeNavigate,
		hScrollLeft = 0,
		visibleWidth = 200
	}: Props = $props();

	let canvasEl: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D | null = $state(null);
	let dpr = $state(1);
	let scale = $state(1.0);
	let baseImageData: ImageData | null = null;
	let imageDataBuffer: ImageData | null = null;
	let lastRenderedStart = 0;
	let renderGen = 0;
	let renderInFlight = false;
	let pendingGen: number | null = null;

	// Scaled values (match Canvas 2D's sc-based scaling)
	let sLaneWidth = $derived(laneWidth * scale);
	let sNodeRadius = $derived(nodeRadius * scale);
	let sPadding = $derived(PADDING_LEFT * scale);

	let containerWidth = $derived(Math.round(visibleWidth));
	let containerHeight = $derived(Math.round((visibleEnd - visibleStart) * rowHeight));

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	let hoveredEdgeIdx = $state<number | null>(null);
	let selectedEdgeIdx = $state<number | null>(null);

	let visibleEdgeData = $derived(
		computeVisibleEdgeCoords(
			layout,
			visibleStart,
			visibleEnd,
			rowHeight,
			sLaneWidth,
			sPadding,
			$arrowGapThreshold
		)
	);

	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);

	// nodeHitMap is intentionally non-reactive — only used in click handler.
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	let nodeHitMap = new Map<string, { x: number; y: number; radius: number }>();

	function buildEdges(segments: VisibleEdgeSegment[]): RenderEdge[] {
		return segments
			.filter((seg) => !isCrossColumn(seg.edge) && seg.coords.wpPx.length === 0)
			.map((seg) => {
				const e = seg.edge;
				return {
					from_row: seg.fromRow,
					from_col: seg.fromCol,
					to_row: seg.toRow,
					to_col: seg.toCol,
					color_r: e.color.r,
					color_g: e.color.g,
					color_b: e.color.b,
					is_dimmed: e.is_dimmed,
					edge_type: 'Straight' as const,
					edge_style: e.edge_style
				};
			});
	}

	let pendingRender = 0;

	function scheduleRender() {
		cancelAnimationFrame(pendingRender);
		pendingRender = requestAnimationFrame(() => {
			const gen = ++renderGen;
			if (renderInFlight) {
				pendingGen = gen;
				return;
			}
			doRender(gen);
		});
	}

	async function doRender(gen: number) {
		if (!canvasEl) return;
		if (gen !== renderGen) return;
		renderInFlight = true;
		try {
			dpr = devicePixelRatio || 1;
			const width = Math.round(visibleWidth);
			const height = Math.round((visibleEnd - visibleStart) * rowHeight);
			if (width < 1 || height < 1) return;

			const physW = Math.round(width * dpr);
			const physH = Math.round(height * dpr);

			if (canvasEl.width !== physW || canvasEl.height !== physH) {
				canvasEl.width = physW;
				canvasEl.height = physH;
				canvasEl.style.width = width + 'px';
				canvasEl.style.height = height + 'px';
				imageDataBuffer = null;
			}

			// Rebuild node hit map (nodes are rendered in overlay, not GPU)
			nodeHitMap = new Map();
			const minCol = Math.max(0, Math.floor((hScrollLeft - sPadding) / sLaneWidth) - 1);
			const maxCol = Math.ceil((hScrollLeft + visibleWidth) / sLaneWidth) + 1;
			for (const n of layout.nodes) {
				if (n.row < visibleStart || n.row > visibleEnd) continue;
				if (n.column < minCol || n.column > maxCol) continue;
				nodeHitMap.set(n.oid, {
					x: columnCenterX(n.column, sLaneWidth, sPadding),
					y: nodeCenterY(n.row, visibleStart, rowHeight),
					radius: sNodeRadius
				});
			}

			// GPU renders only same-column straight edges (no waypoints, no cross-column)
			const input: RenderGraphInput = {
				width: physW,
				height: physH,
				scale: dpr,
				visible_start: visibleStart,
				visible_end: visibleEnd,
				h_scroll_left: hScrollLeft * scale,
				total_columns: layout.total_columns,
				row_height: rowHeight,
				lane_width: sLaneWidth,
				padding_left: sPadding,
				node_radius: sNodeRadius,
				nodes: [],
				edges: buildEdges(visibleEdgeData)
			};

			const pixels = await invoke('render_graph', { input });
			if (gen !== renderGen) return;
			if (!canvasEl) return;
			ctx ??= canvasEl.getContext('2d');
			if (!ctx) return;

			if (!imageDataBuffer || imageDataBuffer.width !== physW || imageDataBuffer.height !== physH) {
				imageDataBuffer = ctx.createImageData(physW, physH);
			}
			const data = imageDataBuffer.data;
			const src =
				pixels instanceof Uint8Array
					? pixels
					: pixels instanceof ArrayBuffer
						? new Uint8Array(pixels)
						: new Uint8Array(pixels as number[]);
			const len = Math.min(src.length, data.length);
			data.set(src.subarray(0, len));
			canvasEl.style.transform = '';
			ctx.putImageData(imageDataBuffer, 0, 0);
			baseImageData = imageDataBuffer;
			lastRenderedStart = visibleStart;

			overdrawOverlay();
		} catch (err) {
			console.warn('wgpu render failed, falling back:', err);
		} finally {
			renderInFlight = false;
			if (pendingGen !== null) {
				const next = pendingGen;
				pendingGen = null;
				doRender(next);
			}
		}
	}

	function drawOverlayEdges() {
		if (!ctx) return;
		for (const seg of visibleEdgeData) {
			const { edge, idx, coords } = seg;
			if (!isCrossColumn(edge) && coords.wpPx.length === 0) continue;

			const color = colorToCSS(edge.color);
			const isSelected = idx === selectedEdgeIdx;
			const isHovered = idx === hoveredEdgeIdx && !isSelected;
			const alpha = edge.is_dimmed ? 0.35 : isSelected ? 1.0 : isHovered ? 0.9 : 0.8;

			ctx.globalAlpha = alpha;
			ctx.strokeStyle = color;
			if (edge.edge_style === 'Dashed') {
				ctx.setLineDash([6, 3]);
			} else if (edge.edge_style === 'Dotted') {
				ctx.setLineDash([2, 3]);
			} else {
				ctx.setLineDash([]);
			}
			ctx.lineWidth = isSelected ? 3.5 : isHovered ? 2.5 : 1.5;
			ctx.beginPath();
			ctx.moveTo(coords.x1, coords.y1);
			for (const wp of coords.wpPx) {
				ctx.lineTo(wp.x, wp.y);
			}
			ctx.lineTo(coords.x2, coords.y2);
			ctx.stroke();
			ctx.setLineDash([]);
		}
		ctx.globalAlpha = 1.0;
	}

	function drawOverlayArrows() {
		if (!ctx) return;
		for (const seg of visibleEdgeData) {
			const { edge, idx, coords, arrow } = seg;
			if (arrow === null) continue;

			const color = colorToCSS(edge.color);
			const isSelected = idx === selectedEdgeIdx;
			const isHovered = idx === hoveredEdgeIdx && !isSelected;
			const alpha = edge.is_dimmed ? 0.35 : isSelected ? 1.0 : isHovered ? 0.9 : 0.8;

			if (arrow === 'down') {
				const headDir = coords.y2 > coords.y1 ? 'down' : 'up';
				drawArrowHead(ctx, coords.x2, coords.y2, color, headDir, alpha);
			} else if (arrow === 'up') {
				const headDir = coords.y2 > coords.y1 ? 'up' : 'down';
				drawArrowHead(ctx, coords.x1, coords.y1, color, headDir, alpha);
			}
		}
	}

	function drawOverlayNode(node: NodePosition) {
		if (!ctx) return;
		const x = columnCenterX(node.column, sLaneWidth, sPadding);
		const y = nodeCenterY(node.row, visibleStart, rowHeight);

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
			const stashIdxMap = new Map(layout.stash_markers.map((s) => [s.stash_oid, s.stash_index]));
			const idx = stashIdxMap.get(node.oid);
			if (idx !== undefined) {
				ctx.font = `${10 * scale}px monospace`;
				ctx.fillStyle = STASH_COLOR;
				ctx.fillText(`S${idx}`, x + sNodeRadius + 2 * scale, y + 3 * scale);
			}
		}
		ctx.globalAlpha = 1.0;
	}

	function drawOverlayNodes() {
		if (!ctx) return;
		for (const node of layout.nodes) {
			if (node.row < visibleStart || node.row > visibleEnd) continue;
			drawOverlayNode(node);
		}
	}

	function drawOverlayHighlights() {
		if (!ctx) return;
		for (const seg of visibleEdgeData) {
			const { edge, idx, coords, arrow, isEdgeStart, isEdgeEnd } = seg;
			const color = colorToCSS(edge.color);

			if (idx === selectedEdgeIdx) {
				drawEdgeHighlight(ctx, coords, color, 3.5);
				drawEdgeEndpoints(ctx, coords, color, sNodeRadius, arrow, isEdgeStart, isEdgeEnd);
			} else if (idx === hoveredEdgeIdx) {
				drawEdgeHighlight(ctx, coords, color, 2.5);
				drawEdgeEndpoints(ctx, coords, color, sNodeRadius, arrow, isEdgeStart, isEdgeEnd);
			}
		}
	}

	function overdrawOverlay() {
		if (!ctx) return;
		ctx.setTransform(dpr, 0, 0, dpr, -hScrollLeft * scale * dpr, 0);
		drawOverlayEdges();
		drawOverlayArrows();
		drawOverlayNodes();
		drawOverlayHighlights();
	}

	$effect(() => {
		void visibleStart;
		void visibleEnd;
		void layout;
		void selectedOid;
		void comparisonOid;
		void rowHeight;
		void laneWidth;
		void nodeRadius;
		void hScrollLeft;
		void visibleWidth;
		void scale;

		if (canvasEl) {
			const rowDelta = visibleStart - lastRenderedStart;
			const viewportRows = visibleEnd - visibleStart;
			if (baseImageData && Math.abs(rowDelta) > 0 && Math.abs(rowDelta) < viewportRows) {
				canvasEl.style.transform = `translateY(${-rowDelta * rowHeight}px)`;
				baseImageData = null;
			} else if (ctx) {
				canvasEl.style.transform = '';
				ctx.setTransform(1, 0, 0, 1, 0, 0);
				ctx.clearRect(0, 0, canvasEl.width, canvasEl.height);
				baseImageData = null;
			}
		}

		scheduleRender();
	});

	$effect(() => {
		void hoveredEdgeIdx;
		void selectedEdgeIdx;
		if (!ctx || !baseImageData) return;
		if (baseImageData.width !== ctx.canvas.width || baseImageData.height !== ctx.canvas.height)
			return;
		ctx.putImageData(baseImageData, 0, 0);
		overdrawOverlay();
	});

	$effect(() => {
		return () => cancelAnimationFrame(pendingRender);
	});

	function handleWheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		const delta = e.deltaY > 0 ? -0.1 : 0.1;
		scale = Math.max(0.5, Math.min(2.0, +(scale + delta).toFixed(2)));
	}

	function handleEdgeClick(mx: number, my: number): boolean {
		for (const { edge, idx, coords, arrow } of visibleEdgeData) {
			if (edgeHitTest(mx, my, coords, EDGE_HIT_TOLERANCE)) {
				if (selectedEdgeIdx === idx) {
					const farOid = edgeFarOid(edge, layout, selectedOid ?? null, arrow);
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
		const rect = canvasEl.getBoundingClientRect();
		const mx = e.clientX - rect.left + hScrollLeft * scale;
		const my = e.clientY - rect.top;
		for (const [oid, pos] of nodeHitMap) {
			if (nodeHitTest(mx, my, pos.x, pos.y, pos.radius)) {
				selectedEdgeIdx = null;
				onSelect?.(oid, e.ctrlKey || e.metaKey);
				return;
			}
		}
		if (!handleEdgeClick(mx, my)) {
			selectedEdgeIdx = null;
		}
	}

	function handleMouseMove(e: MouseEvent) {
		const rect = canvasEl.getBoundingClientRect();
		const mx = e.clientX - rect.left + hScrollLeft * scale;
		const my = e.clientY - rect.top;
		const row = Math.floor(my / rowHeight) + visibleStart;
		const hitRadius = sNodeRadius + 4;

		for (const n of layout.nodes) {
			if (n.row === row) {
				const nx = columnCenterX(n.column, sLaneWidth, sPadding);
				const ny = nodeCenterY(n.row, visibleStart, rowHeight);
				if (Math.abs(mx - nx) < hitRadius && Math.abs(my - ny) < hitRadius) {
					if (n.is_stash) {
						const stash = layout.stash_markers.find((s) => s.stash_oid === n.oid);
						tooltip = {
							x: e.clientX - rect.left + 12,
							y: e.clientY - rect.top - 8,
							text: stash?.message ?? n.oid.substring(0, 7) + ' stash'
						};
					} else {
						const ci = commitMap.get(n.oid);
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
			if (edgeHitTest(mx, my, coords, EDGE_HIT_TOLERANCE)) {
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
</script>

<div class="relative" style="width: {containerWidth}px; height: {containerHeight}px;">
	<canvas
		bind:this={canvasEl}
		onclick={handleClick}
		onmousemove={handleMouseMove}
		onmouseleave={handleMouseLeave}
		onwheel={handleWheel}
		class="block absolute inset-0"
		style="image-rendering: auto; width: {containerWidth}px; height: {containerHeight}px;"
	></canvas>
	{#if tooltip}
		<div
			class="pointer-events-none absolute z-50 max-w-[250px] rounded bg-gray-800 px-2 py-1 text-xs text-gray-200 shadow-lg border border-gray-700"
			style="left: {tooltip.x}px; top: {tooltip.y}px;"
		>
			{tooltip.text}
		</div>
	{/if}
</div>
