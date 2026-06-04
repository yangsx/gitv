<script lang="ts">
	import type { FileHistoryEntry } from '$lib/bindings/types';
	import { getFileHistory } from '$lib/bindings/commands';

	interface Props {
		repoPath: string;
		filePath: string;
		onclose: () => void;
	}

	let { repoPath, filePath, onclose }: Props = $props();

	let entries = $state<FileHistoryEntry[]>([]);
	let loading = $state(true);

	$effect(() => {
		loadHistory();
	});

	async function loadHistory() {
		loading = true;
		try {
			entries = await getFileHistory(repoPath, filePath);
		} catch {
			entries = [];
		} finally {
			loading = false;
		}
	}

	function formatTime(iso: string): string {
		try {
			return new Date(iso).toLocaleDateString(undefined, {
				month: 'short',
				day: 'numeric',
				year: 'numeric'
			});
		} catch {
			return '';
		}
	}
</script>

<div class="flex h-full flex-col bg-gray-900">
	<div class="flex items-center gap-2 border-b border-gray-700 px-4 py-2">
		<h3 class="truncate font-mono text-sm text-gray-300">History: {filePath}</h3>
		<button
			class="ml-auto shrink-0 rounded px-2 py-1 text-xs text-gray-400 hover:bg-gray-800 hover:text-gray-200"
			onclick={onclose}
		>
			Close
		</button>
	</div>
	<div class="flex-1 overflow-y-auto">
		{#if loading}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">
				Loading history...
			</div>
		{:else if entries.length === 0}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">
				No history found
			</div>
		{:else}
			{#each entries as entry (entry.commit_oid)}
				<div class="border-b border-gray-800 px-4 py-2 text-xs hover:bg-gray-800/50">
					<div class="flex items-center gap-2">
						<span class="font-mono text-gray-500">{entry.commit_oid.slice(0, 8)}</span>
						<span class="flex-1 truncate text-gray-300">{entry.summary}</span>
						<span class="shrink-0 text-gray-500">{entry.author.name}</span>
						<span class="shrink-0 text-gray-600">{formatTime(entry.time)}</span>
					</div>
					{#if entry.old_path}
						<div class="mt-1 text-gray-500">
							Renamed from
							<span class="font-mono text-blue-400">{entry.old_path}</span>
						</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>
