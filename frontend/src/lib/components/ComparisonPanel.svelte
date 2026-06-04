<script lang="ts">
	import type { DiffSummary } from '$lib/bindings/types';
	import { getDiff } from '$lib/bindings/commands';

	interface Props {
		repoPath: string;
		fromOid: string;
		toOid: string;
	}

	let { repoPath, fromOid, toOid }: Props = $props();

	let summary = $state<DiffSummary | null>(null);
	let loading = $state(true);
	let selectedFile = $state<string | null>(null);

	const CHANGE_COLORS: Record<string, string> = {
		Added: 'text-green-400',
		Deleted: 'text-red-400',
		Modified: 'text-yellow-400',
		Renamed: 'text-blue-400',
		Copied: 'text-purple-400',
		SubmoduleUpdated: 'text-orange-400'
	};

	$effect(() => {
		loadDiff();
	});

	async function loadDiff() {
		loading = true;
		try {
			summary = await getDiff(repoPath, fromOid, toOid);
		} catch {
			summary = null;
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex h-full">
	{#if loading}
		<div class="flex items-center justify-center w-full text-sm text-gray-500">
			Loading comparison...
		</div>
	{:else if summary}
		<div class="w-64 shrink-0 overflow-y-auto border-r border-gray-700 bg-gray-900/50">
			<div class="border-b border-gray-700 px-3 py-2">
				<h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider">
					{summary.stats.files_changed} files changed
					<span class="text-green-500">+{summary.stats.additions}</span>
					<span class="text-red-500">-{summary.stats.deletions}</span>
				</h3>
			</div>
			{#each summary.files as file (file.path)}
				<button
					class="flex w-full items-center gap-2 border-b border-gray-800 px-3 py-1.5 text-left text-xs hover:bg-gray-800/70 {selectedFile ===
					file.path
						? 'bg-gray-800'
						: ''}"
					onclick={() => (selectedFile = file.path === selectedFile ? null : file.path)}
				>
					<span class="w-4 text-center font-bold {CHANGE_COLORS[file.change_type] ?? ''}">
						{file.change_type[0]}
					</span>
					<span class="flex-1 truncate font-mono text-gray-300">{file.path}</span>
					{#if !file.is_binary}
						<span class="shrink-0 font-mono text-[10px]">
							<span class="text-green-500">{file.additions > 0 ? '+' + file.additions : ''}</span>
							<span class="text-red-500">{file.deletions > 0 ? '-' + file.deletions : ''}</span>
						</span>
					{:else}
						<span class="text-[10px] text-gray-500">binary</span>
					{/if}
				</button>
			{/each}
		</div>

		<div class="flex-1 overflow-y-auto bg-gray-900">
			{#if selectedFile}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					Select a file in the diff viewer to see details
				</div>
			{:else}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					Select a file to view diff
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex items-center justify-center w-full text-sm text-gray-500">
			Failed to load comparison
		</div>
	{/if}
</div>
