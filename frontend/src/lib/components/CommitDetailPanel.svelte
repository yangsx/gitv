<script lang="ts">
	import type { CommitDetails, FileDiff } from '$lib/bindings/types';
	import { getFileDiff } from '$lib/bindings/commands';
	import DiffViewer from './DiffViewer.svelte';

	interface Props {
		details: CommitDetails;
		repoPath: string;
	}

	let { details, repoPath }: Props = $props();

	let selectedFile = $state<string | null>(null);
	let fileDiff = $state<FileDiff | null>(null);
	let loadingDiff = $state(false);

	const CHANGE_COLORS: Record<string, string> = {
		Added: 'text-green-400',
		Deleted: 'text-red-400',
		Modified: 'text-yellow-400',
		Renamed: 'text-blue-400',
		Copied: 'text-purple-400',
		SubmoduleUpdated: 'text-orange-400'
	};

	const CHANGE_LETTERS: Record<string, string> = {
		Added: 'A',
		Deleted: 'D',
		Modified: 'M',
		Renamed: 'R',
		Copied: 'C',
		SubmoduleUpdated: 'S'
	};

	async function loadFileDiff(path: string) {
		if (selectedFile === path) {
			selectedFile = null;
			fileDiff = null;
			return;
		}
		selectedFile = path;
		loadingDiff = true;
		try {
			const parentOid = details.info.parent_oids[0] ?? null;
			fileDiff = await getFileDiff(repoPath, parentOid, details.info.oid, path);
		} catch {
			fileDiff = null;
		} finally {
			loadingDiff = false;
		}
	}
</script>

<div class="flex h-full">
	<div class="w-64 shrink-0 overflow-y-auto border-r border-gray-700 bg-gray-900/50">
		<div class="border-b border-gray-700 px-3 py-2">
			<h3 class="text-xs font-semibold text-gray-400 uppercase tracking-wider">Changed Files</h3>
		</div>
		{#each details.changed_files as file}
			<button
				class="flex w-full items-center gap-2 border-b border-gray-800 px-3 py-1.5 text-left text-xs hover:bg-gray-800/70 {selectedFile ===
				file.path
					? 'bg-gray-800'
					: ''}"
				onclick={() => loadFileDiff(file.path)}
			>
				<span class="w-4 text-center font-bold {CHANGE_COLORS[file.change_type] ?? ''}">
					{CHANGE_LETTERS[file.change_type] ?? '?'}
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
		{#if details.changed_files.length === 0}
			<div class="px-3 py-4 text-xs text-gray-500">No changed files</div>
		{/if}
	</div>

	<div class="flex-1 overflow-y-auto bg-gray-900">
		{#if loadingDiff}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">Loading diff...</div>
		{:else if fileDiff}
			<div class="border-b border-gray-700 px-4 py-2">
				<h3 class="font-mono text-sm text-gray-300">{fileDiff.path}</h3>
			</div>
			{#if fileDiff.is_binary}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					Binary file (not displayed)
				</div>
			{:else if fileDiff.hunks.length === 0}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					No content changes
				</div>
			{:else}
				<div class="overflow-x-auto p-2">
					<DiffViewer hunks={fileDiff.hunks} />
				</div>
			{/if}
		{:else}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">
				Select a file to view diff
			</div>
		{/if}
	</div>
</div>
