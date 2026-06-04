<script lang="ts">
	import type { DiffSummary, FileDiff } from '$lib/bindings/types';
	import { getDiff, getFileDiff } from '$lib/bindings/commands';
	import DiffViewer from './DiffViewer.svelte';

	interface Props {
		repoPath: string;
		fromOid: string;
		toOid: string;
	}

	let { repoPath, fromOid, toOid }: Props = $props();

	let summary = $state<DiffSummary | null>(null);
	let loading = $state(true);
	let selectedFile = $state<string | null>(null);
	let fileDiff = $state<FileDiff | null>(null);
	let loadingDiff = $state(false);
	let fullDiff = $state(false);
	let diffError = $state<string | null>(null);

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
		selectedFile = null;
		fileDiff = null;
		try {
			summary = await getDiff(repoPath, fromOid, toOid);
		} catch {
			summary = null;
		} finally {
			loading = false;
		}
	}

	async function loadFileDiff(path: string) {
		if (selectedFile === path) {
			selectedFile = null;
			fileDiff = null;
			return;
		}
		selectedFile = path;
		fullDiff = false;
		diffError = null;
		loadingDiff = true;
		try {
			fileDiff = await getFileDiff(repoPath, fromOid, toOid, path);
		} catch (e: unknown) {
			diffError = e instanceof Error ? e.message : String(e);
		} finally {
			loadingDiff = false;
		}
	}

	async function loadFullDiff() {
		fullDiff = true;
		if (!selectedFile) return;
		diffError = null;
		loadingDiff = true;
		try {
			fileDiff = await getFileDiff(
				repoPath,
				fromOid,
				toOid,
				selectedFile,
				undefined,
				undefined,
				true
			);
		} catch (e: unknown) {
			diffError = e instanceof Error ? e.message : String(e);
		} finally {
			loadingDiff = false;
		}
	}
</script>

<div class="flex h-full">
	{#if loading}
		<div
			class="flex items-center justify-center w-full text-sm text-gray-500"
			role="status"
			aria-live="polite"
		>
			Loading comparison...
		</div>
	{:else if summary}
		<div
			class="w-64 shrink-0 overflow-y-auto border-r border-gray-700 bg-gray-900/50"
			role="list"
			aria-label="Changed files"
		>
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
					aria-label="{file.path}, {file.change_type}"
					aria-pressed={selectedFile === file.path}
					onclick={() => loadFileDiff(file.path)}
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
			{#if loadingDiff}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					Loading diff...
				</div>
			{:else if diffError}
				<div class="flex items-center justify-center py-8 text-sm text-red-400">
					{diffError}
				</div>
			{:else if fileDiff}
				{#if fileDiff.is_submodule}
					<div class="flex items-center justify-center py-8 text-sm text-orange-400">
						{fileDiff.hunks
							.flatMap((h) => h.lines)
							.map((l) => ('Addition' in l ? l.Addition.content : ''))
							.filter(Boolean)
							.join(' ')}
					</div>
				{:else if fileDiff.is_binary}
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
					{#if fileDiff.truncated_at != null}
						<div class="flex items-center justify-center gap-3 border-t border-gray-700 py-2">
							<span class="text-xs text-gray-500">
								Diff truncated at {fileDiff.truncated_at} lines
							</span>
							<button
								class="rounded bg-blue-700 px-3 py-1 text-xs text-white hover:bg-blue-600"
								onclick={loadFullDiff}
							>
								Show full diff
							</button>
						</div>
					{/if}
				{/if}
			{:else}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500" role="status">
					Select a file to view diff
				</div>
			{/if}
		</div>
	{:else}
		<div class="flex items-center justify-center w-full text-sm text-gray-500" role="alert">
			Failed to load comparison
		</div>
	{/if}
</div>
