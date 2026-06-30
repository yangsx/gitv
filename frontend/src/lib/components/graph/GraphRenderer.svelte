<script lang="ts">
	import { renderer } from '$lib/stores/preferences';
	import type { GraphLayout, CommitInfo } from '$lib/bindings/types';
	import { WGPU_MAX_TEXTURE_DIMENSION } from '$lib/constants';
	import CommitGraph from '../CommitGraph.svelte';
	import WgpuGraph from './WgpuGraph.svelte';

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
		visibleWidth = 200
	}: Props = $props();

	let useWgpu = $derived($renderer === 'wgpu');
	let canUseWgpu = $derived.by(() => {
		if (!useWgpu) return false;
		const dpr = window.devicePixelRatio || 1;
		const physW = Math.round(visibleWidth * dpr);
		return physW <= WGPU_MAX_TEXTURE_DIMENSION;
	});
</script>

{#if canUseWgpu}
	<WgpuGraph
		{layout}
		{commits}
		{rowHeight}
		{laneWidth}
		{nodeRadius}
		{visibleStart}
		{visibleEnd}
		{selectedOid}
		{comparisonOid}
		{onSelect}
		{onEdgeNavigate}
		{visibleWidth}
	/>
{:else}
	<CommitGraph
		{layout}
		{commits}
		{rowHeight}
		{laneWidth}
		{nodeRadius}
		{visibleStart}
		{visibleEnd}
		{selectedOid}
		{comparisonOid}
		{onSelect}
		{onEdgeNavigate}
		{visibleWidth}
	/>
{/if}
