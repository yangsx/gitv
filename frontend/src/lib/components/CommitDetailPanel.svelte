<script lang="ts">
	import type { CommitDetails, FileDiff, FileTreeNode } from '$lib/bindings/types';
	import { getFileDiff, getFileTree } from '$lib/bindings/commands';
	import DiffViewer from './DiffViewer.svelte';
	import FileTree from './FileTree.svelte';

	interface Props {
		details: CommitDetails;
		repoPath: string;
	}

	let { details, repoPath }: Props = $props();

	let selectedFile = $state<string | null>(null);
	let fileDiff = $state<FileDiff | null>(null);
	let loadingDiff = $state(false);
	let diffMode = $state<'normal' | 'word-diff' | 'stat-only'>('normal');
	let whitespaceMode = $state<
		'none' | 'ignore-space-change' | 'ignore-all-space' | 'ignore-blank-lines'
	>('none');
	let activeTab = $state<'changes' | 'tree'>('changes');
	let fileTree = $state<FileTreeNode | null>(null);
	let loadingTree = $state(false);

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
			fileDiff = await getFileDiff(
				repoPath,
				parentOid,
				details.info.oid,
				path,
				diffMode,
				whitespaceMode
			);
		} catch {
			fileDiff = null;
		} finally {
			loadingDiff = false;
		}
	}

	async function loadFileTree() {
		if (fileTree) return;
		loadingTree = true;
		try {
			fileTree = await getFileTree(repoPath, details.info.oid);
		} catch {
			fileTree = null;
		} finally {
			loadingTree = false;
		}
	}

	function switchTab(tab: 'changes' | 'tree') {
		activeTab = tab;
		if (tab === 'tree') loadFileTree();
	}

	async function refreshDiff() {
		if (!selectedFile) return;
		loadingDiff = true;
		try {
			const parentOid = details.info.parent_oids[0] ?? null;
			fileDiff = await getFileDiff(
				repoPath,
				parentOid,
				details.info.oid,
				selectedFile,
				diffMode,
				whitespaceMode
			);
		} catch {
			fileDiff = null;
		} finally {
			loadingDiff = false;
		}
	}
</script>

<div class="flex h-full">
	<div class="w-64 shrink-0 flex flex-col border-r border-gray-700 bg-gray-900/50">
		<div class="flex border-b border-gray-700">
			<button
				class="flex-1 px-3 py-2 text-xs font-semibold uppercase tracking-wider transition-colors {activeTab ===
				'changes'
					? 'text-gray-200 border-b-2 border-blue-500'
					: 'text-gray-500 hover:text-gray-300'}"
				onclick={() => switchTab('changes')}
			>
				Changes
			</button>
			<button
				class="flex-1 px-3 py-2 text-xs font-semibold uppercase tracking-wider transition-colors {activeTab ===
				'tree'
					? 'text-gray-200 border-b-2 border-blue-500'
					: 'text-gray-500 hover:text-gray-300'}"
				onclick={() => switchTab('tree')}
			>
				Files
			</button>
		</div>

		<div class="flex-1 overflow-y-auto">
			{#if activeTab === 'changes'}
				{#each details.changed_files as file (file.path)}
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
			{:else if loadingTree}
				<div class="px-3 py-4 text-xs text-gray-500">Loading tree...</div>
			{:else if fileTree}
				<FileTree node={fileTree} {repoPath} />
			{:else}
				<div class="px-3 py-4 text-xs text-gray-500">No file tree</div>
			{/if}
		</div>
	</div>

	<div class="flex-1 flex flex-col overflow-hidden bg-gray-900">
		{#if loadingDiff}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">Loading diff...</div>
		{:else if fileDiff}
			<div class="flex items-center gap-2 border-b border-gray-700 px-4 py-2">
				<h3 class="font-mono text-sm text-gray-300">{fileDiff.path}</h3>
				<div class="ml-auto flex items-center gap-2">
					<select
						class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300"
						bind:value={diffMode}
						onchange={refreshDiff}
					>
						<option value="normal">Normal</option>
						<option value="word-diff">Word Diff</option>
						<option value="stat-only">Stat Only</option>
					</select>
					{#if diffMode !== 'stat-only'}
						<select
							class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300"
							bind:value={whitespaceMode}
							onchange={refreshDiff}
						>
							<option value="none">Show Whitespace</option>
							<option value="ignore-space-change">Ignore Space Change</option>
							<option value="ignore-all-space">Ignore All Space</option>
							<option value="ignore-blank-lines">Ignore Blank Lines</option>
						</select>
					{/if}
				</div>
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
				<div class="flex-1 overflow-auto p-2">
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
