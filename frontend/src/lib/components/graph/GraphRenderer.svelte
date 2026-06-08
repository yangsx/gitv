<script lang="ts">
	import { renderer } from '$lib/stores/preferences';
	import type { GraphLayout, CommitInfo } from '$lib/bindings/types';
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
		onSelect
	}: Props = $props();

	let useWgpu = $derived($renderer === 'wgpu');
</script>

{#if useWgpu}
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
	/>
{/if}
