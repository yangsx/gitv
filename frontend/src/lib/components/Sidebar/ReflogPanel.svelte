<script lang="ts">
	import type { ReflogEntry } from '$lib/bindings/types';
	import { getReflog } from '$lib/bindings/commands';

	let {
		repoPath,
		onentryselect
	}: {
		repoPath: string;
		onentryselect?: (oid: string) => void;
	} = $props();

	let entries = $state<ReflogEntry[]>([]);
	let loading = $state(true);
	let filterOp = $state('');
	let selectedRef = $state('HEAD');

	$effect(() => {
		void repoPath;
		void selectedRef;
		loadReflog();
	});

	async function loadReflog() {
		loading = true;
		try {
			entries = await getReflog(
				repoPath,
				selectedRef === 'HEAD' ? undefined : `refs/heads/${selectedRef}`
			);
		} catch {
			entries = [];
		} finally {
			loading = false;
		}
	}

	function formatTime(timeStr: string): string {
		try {
			const d = new Date(timeStr);
			return (
				d.toLocaleDateString() +
				' ' +
				d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
			);
		} catch {
			return '';
		}
	}

	let filteredEntries = $derived(
		filterOp
			? entries.filter((e) => e.message.toLowerCase().includes(filterOp.toLowerCase()))
			: entries
	);
</script>

<div class="space-y-2">
	<div class="flex gap-1">
		<select
			class="rounded border border-gray-700 bg-gray-800 px-1 py-0.5 text-xs text-gray-300"
			bind:value={selectedRef}
		>
			<option value="HEAD">HEAD</option>
		</select>
		<input
			type="text"
			class="flex-1 rounded border border-gray-700 bg-gray-800 px-1.5 py-0.5 text-xs text-gray-300 placeholder-gray-500"
			placeholder="Filter operations..."
			bind:value={filterOp}
		/>
	</div>

	{#if loading}
		<div class="text-gray-500">Loading reflog...</div>
	{:else if filteredEntries.length === 0}
		<div class="text-gray-500 italic">No reflog entries</div>
	{:else}
		<div class="space-y-0.5">
			{#each filteredEntries.slice(0, 100) as entry (entry.oid + entry.message)}
				<button
					class="w-full rounded px-1.5 py-1 text-left hover:bg-gray-800"
					onclick={() => onentryselect?.(entry.oid)}
				>
					<div class="flex items-center gap-1">
						<span class="font-mono text-gray-400">{entry.oid.slice(0, 7)}</span>
						<span class="truncate text-gray-300">{entry.message.slice(0, 50)}</span>
					</div>
					<div class="mt-0.5 text-gray-500">{entry.author.name} · {formatTime(entry.time)}</div>
				</button>
			{/each}
		</div>
	{/if}
</div>
