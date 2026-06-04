<script lang="ts">
	import type { CommitInfo, GraphLayout } from '$lib/bindings/types';
	import CommitGraph from './CommitGraph.svelte';
	import CommitRow from './CommitRow.svelte';

	interface Props {
		commits: CommitInfo[];
		layout: GraphLayout;
		selectedOid: string | null;
		matchingOids?: Set<string>;
		onSelect: (_oid: string) => void;
		rowHeight?: number;
		graphWidth?: number;
	}

	let {
		commits,
		layout,
		selectedOid,
		matchingOids,
		onSelect,
		rowHeight = 28,
		graphWidth = 200
	}: Props = $props();

	let containerEl: HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(800);

	const BUFFER = 10;

	let commitsByOid = $derived(new Map(commits.map((c) => [c.oid, c])));

	let orderedCommits = $derived(
		layout.nodes.map((n) => commitsByOid.get(n.oid)).filter((c): c is CommitInfo => c !== undefined)
	);

	let visibleStart = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - BUFFER));
	let visibleEnd = $derived(
		Math.min(orderedCommits.length, Math.ceil((scrollTop + containerHeight) / rowHeight) + BUFFER)
	);
	let totalHeight = $derived(orderedCommits.length * rowHeight);
	let visibleCommits = $derived(orderedCommits.slice(visibleStart, visibleEnd));

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
					<CommitRow
						{commit}
						isSelected={commit.oid === selectedOid}
						isDimmed={matchingOids ? !matchingOids.has(commit.oid) : false}
						onclick={onSelect}
					/>
				{/each}
			</div>
		</div>
	</div>
</div>
