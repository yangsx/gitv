<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type {
		GraphLayout,
		RenderGraphInput,
		RenderNode,
		RenderEdge,
		CommitInfo
	} from '$lib/bindings/types';
	import {
		columnCenterX,
		nodeCenterY,
		nodeHitTest,
		colorToCSS,
		hasArrowGap,
		ARROW_SEGMENT_LENGTH,
		SELECT_RGB,
		COMPARISON_RGB
	} from '$lib/graph/graph-math';
	import {
		edgeHitTest,
		edgeFarOid,
		drawEdgeHighlight,
		drawEdgeEndpoints,
		drawArrowHead,
		type VisibleEdgeSegment
	} from '$lib/graph/edge-interaction';
	import {
		GRAPH_PADDING_LEFT as PADDING_LEFT,
		GRAPH_EDGE_HIT_TOLERANCE as EDGE_HIT_TOLERANCE
	} from '$lib/constants';
	import { arrowGapThreshold } from '$lib/stores/repository';

	const EDGE_BUCKET_SIZE = 64;

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
	let baseImageData: ImageData | null = null;
	let imageDataBuffer: ImageData | null = null;
	let lastRenderedStart = 0;
	let renderGen = 0;
	let renderInFlight = false;
	let pendingGen: number | null = null;

	let containerWidth = $derived(Math.round(visibleWidth));
	let containerHeight = $derived(Math.round((visibleEnd - visibleStart) * rowHeight));

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	// Pre-built edge bucket index for O(1) viewport lookup.
	// Each segment is bucketed by its minRow so we only scan relevant buckets.
	type BucketEntry = {
		edgeIdx: number;
		fromRow: number;
		toRow: number;
		arrow: 'down' | 'up' | null;
	};

	let edgeBuckets = $derived.by<BucketEntry[][]>(() => {
		const totalRows = layout.nodes.length;
		const numBuckets = Math.max(1, Math.ceil(totalRows / EDGE_BUCKET_SIZE));
		const buckets: BucketEntry[][] = Array.from({ length: numBuckets }, () => []);
		const threshold = $arrowGapThreshold;
		for (let i = 0; i < layout.edges.length; i++) {
			const edge = layout.edges[i];
			if (hasArrowGap(edge, threshold)) {
				const dir = edge.to_row > edge.from_row ? 1 : -1;
				const seg1End = edge.from_row + dir * ARROW_SEGMENT_LENGTH;
				const seg2Start = edge.to_row - dir * ARROW_SEGMENT_LENGTH;
				const e1Min = Math.min(edge.from_row, seg1End);
				const e1Max = Math.max(edge.from_row, seg1End);
				const e2Min = Math.min(seg2Start, edge.to_row);
				const e2Max = Math.max(seg2Start, edge.to_row);
				for (
					let b = Math.floor(e1Min / EDGE_BUCKET_SIZE);
					b <= Math.floor(e1Max / EDGE_BUCKET_SIZE) && b < numBuckets;
					b++
				) {
					if (b >= 0)
						buckets[b].push({ edgeIdx: i, fromRow: edge.from_row, toRow: seg1End, arrow: 'down' });
				}
				for (
					let b = Math.floor(e2Min / EDGE_BUCKET_SIZE);
					b <= Math.floor(e2Max / EDGE_BUCKET_SIZE) && b < numBuckets;
					b++
				) {
					if (b >= 0)
						buckets[b].push({ edgeIdx: i, fromRow: seg2Start, toRow: edge.to_row, arrow: 'up' });
				}
			} else {
				const eMin = Math.min(edge.from_row, edge.to_row);
				const eMax = Math.max(edge.from_row, edge.to_row);
				for (
					let b = Math.floor(eMin / EDGE_BUCKET_SIZE);
					b <= Math.floor(eMax / EDGE_BUCKET_SIZE) && b < numBuckets;
					b++
				) {
					if (b >= 0)
						buckets[b].push({
							edgeIdx: i,
							fromRow: edge.from_row,
							toRow: edge.to_row,
							arrow: null
						});
				}
			}
		}
		return buckets;
	});

	let hoveredEdgeIdx = $state<number | null>(null);
	let selectedEdgeIdx = $state<number | null>(null);

	let visibleEdgeData = $derived.by<VisibleEdgeSegment[]>(() => {
		const startB = Math.max(0, Math.floor(visibleStart / EDGE_BUCKET_SIZE));
		const endB = Math.min(edgeBuckets.length - 1, Math.floor(visibleEnd / EDGE_BUCKET_SIZE));
		const seen: Record<string, true> = {};
		const result: VisibleEdgeSegment[] = [];
		for (let b = startB; b <= endB; b++) {
			for (const entry of edgeBuckets[b]) {
				const key = `${entry.edgeIdx}:${entry.arrow}`;
				if (seen[key]) continue;
				seen[key] = true;
				const edge = layout.edges[entry.edgeIdx];
				const minR = Math.min(entry.fromRow, entry.toRow);
				const maxR = Math.max(entry.fromRow, entry.toRow);
				if (minR > visibleEnd || maxR < visibleStart) continue;
				result.push({
					edge,
					idx: entry.edgeIdx,
					coords: {
						x1: columnCenterX(edge.from_col, laneWidth, PADDING_LEFT),
						y1: nodeCenterY(entry.fromRow, visibleStart, rowHeight),
						x2: columnCenterX(edge.to_col, laneWidth, PADDING_LEFT),
						y2: nodeCenterY(entry.toRow, visibleStart, rowHeight),
						sameColumn: edge.from_col === edge.to_col
					},
					arrow: entry.arrow,
					fromRow: entry.fromRow,
					fromCol: edge.from_col,
					toRow: entry.toRow,
					toCol: edge.to_col
				});
			}
		}
		return result;
	});

	let visibleStashLabels = $derived.by(() => {
		const stashIdxMap = new Map(layout.stash_markers.map((s) => [s.stash_oid, s.stash_index]));
		return layout.nodes
			.filter((n) => n.is_stash && n.row >= visibleStart && n.row <= visibleEnd)
			.map((n) => ({
				index: stashIdxMap.get(n.oid) ?? 0,
				left: Math.round(
					columnCenterX(n.column, laneWidth, PADDING_LEFT) + nodeRadius + 2 - hScrollLeft
				),
				top: Math.round(nodeCenterY(n.row, visibleStart, rowHeight))
			}));
	});

	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);

	// nodeHitMap is intentionally non-reactive — only used in click handler.
	// eslint-disable-next-line svelte/prefer-svelte-reactivity
	let nodeHitMap = new Map<string, { x: number; y: number; radius: number }>();

	function buildNodes(start: number, end: number): RenderNode[] {
		const result: RenderNode[] = [];
		nodeHitMap = new Map();
		const minCol = Math.max(0, Math.floor((hScrollLeft - PADDING_LEFT) / laneWidth) - 1);
		const maxCol = Math.ceil((hScrollLeft + visibleWidth) / laneWidth) + 1;
		for (const n of layout.nodes) {
			if (n.row < start || n.row > end) continue;
			if (n.column < minCol || n.column > maxCol) continue;
			const x = columnCenterX(n.column, laneWidth, PADDING_LEFT);
			const y = nodeCenterY(n.row, start, rowHeight);
			const [cr, cg, cb, ca] = [n.color.r, n.color.g, n.color.b, n.color.a];
			const [sr, sg, sb] = n.oid === comparisonOid ? COMPARISON_RGB : SELECT_RGB;
			result.push({
				row: n.row,
				column: n.column,
				color_r: cr,
				color_g: cg,
				color_b: cb,
				color_a: ca,
				is_dimmed: n.is_dimmed,
				is_selected: n.oid === selectedOid,
				is_comparison: n.oid === comparisonOid,
				is_merge: n.is_merge,
				is_stash: n.is_stash,
				sel_color_r: sr,
				sel_color_g: sg,
				sel_color_b: sb
			});
			nodeHitMap.set(n.oid, { x, y, radius: nodeRadius });
		}
		return result;
	}

	function buildEdges(segments: VisibleEdgeSegment[]): RenderEdge[] {
		return segments.map((seg) => {
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
				edge_type: seg.arrow !== null ? ('Straight' as const) : e.edge_type,
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

			const input: RenderGraphInput = {
				width: physW,
				height: physH,
				scale: dpr,
				visible_start: visibleStart,
				visible_end: visibleEnd,
				h_scroll_left: hScrollLeft,
				total_columns: layout.total_columns,
				row_height: rowHeight,
				lane_width: laneWidth,
				padding_left: PADDING_LEFT,
				node_radius: nodeRadius,
				nodes: buildNodes(visibleStart, visibleEnd),
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

	function overdrawOverlay() {
		if (!ctx) return;
		ctx.setTransform(dpr, 0, 0, dpr, -hScrollLeft * dpr, 0);
		for (const { edge, idx, coords, arrow } of visibleEdgeData) {
			const color = colorToCSS(edge.color);
			const alpha = edge.is_dimmed ? 0.35 : 0.8;

			if (arrow === 'down') {
				const headDir = coords.y2 > coords.y1 ? 'down' : 'up';
				drawArrowHead(ctx, coords.x2, coords.y2, color, headDir, alpha);
			} else if (arrow === 'up') {
				const headDir = coords.y2 > coords.y1 ? 'up' : 'down';
				drawArrowHead(ctx, coords.x1, coords.y1, color, headDir, alpha);
			}

			if (idx === selectedEdgeIdx) {
				drawEdgeHighlight(ctx, coords, color, 3.5);
				drawEdgeEndpoints(ctx, coords, color, nodeRadius, arrow);
			} else if (idx === hoveredEdgeIdx) {
				drawEdgeHighlight(ctx, coords, color, 2.5);
				drawEdgeEndpoints(ctx, coords, color, nodeRadius, arrow);
			}
		}
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
		const mx = e.clientX - rect.left + hScrollLeft;
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
		const mx = e.clientX - rect.left + hScrollLeft;
		const my = e.clientY - rect.top;
		const row = Math.floor(my / rowHeight) + visibleStart;
		const hitRadius = nodeRadius + 4;

		for (const n of layout.nodes) {
			if (n.row === row) {
				const nx = columnCenterX(n.column, laneWidth, PADDING_LEFT);
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
	<div class="absolute inset-0" style="pointer-events: none;">
		{#each visibleStashLabels as s (s.index)}
			<span
				class="absolute text-xs font-bold font-mono leading-none select-none"
				style="left: {s.left}px; top: {s.top}px; transform: translateY(-50%); color: #f59e0b;"
				>S{s.index}</span
			>
		{/each}
	</div>
</div>
