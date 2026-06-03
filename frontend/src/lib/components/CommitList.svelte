<script lang="ts">
	import type { CommitInfo, GraphLayout } from '$lib/bindings/types';
	import CommitGraph from './CommitGraph.svelte';
	import CommitRow from './CommitRow.svelte';

	interface Props {
		commits: CommitInfo[];
		layout: GraphLayout;
		selectedOid: string | null;
		onSelect: (_oid: string) => void;
		rowHeight?: number;
		graphWidth?: number;
	}

	let {
		commits,
		layout,
		selectedOid,
		onSelect,
		rowHeight = 28,
		graphWidth = 200
	}: Props = $props();

	let containerEl: HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(800);

	const BUFFER = 10;

	let visibleStart = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - BUFFER));
	let visibleEnd = $derived(
		Math.min(commits.length, Math.ceil((scrollTop + containerHeight) / rowHeight) + BUFFER)
	);
	let totalHeight = $derived(commits.length * rowHeight);
	let visibleCommits = $derived(commits.slice(visibleStart, visibleEnd));

	function onScroll(e: Event) {
		const el = e.target as HTMLDivElement;
		scrollTop = el.scrollTop;
	}

	function handleResize() {
		if (containerEl) {
			containerHeight = containerEl.clientHeight;
		}
	}

	$effect(() => {
		handleResize();
		const observer = new ResizeObserver(handleResize);
		if (containerEl) observer.observe(containerEl);
		return () => observer.disconnect();
	});
</script>

<div class="flex h-full">
	<div class="shrink-0 overflow-hidden" style="width: {graphWidth}px; height: 100%;">
		<div style="height: {totalHeight}px; position: relative;">
			<div style="transform: translateY({visibleStart * rowHeight}px);">
				<CommitGraph {layout} {rowHeight} {visibleStart} {visibleEnd} />
			</div>
		</div>
	</div>

	<div bind:this={containerEl} class="flex-1 overflow-y-auto" onscroll={onScroll}>
		<div style="height: {totalHeight}px; position: relative;">
			<div style="transform: translateY({visibleStart * rowHeight}px);">
				{#each visibleCommits as commit (commit.oid)}
					<CommitRow {commit} isSelected={commit.oid === selectedOid} onclick={onSelect} />
				{/each}
			</div>
		</div>
	</div>
</div>
