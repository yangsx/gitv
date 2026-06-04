<script lang="ts">
	import type { CommitDetails, FileDiff, FileTreeNode } from '$lib/bindings/types';
	import {
		getFileDiff,
		getFileTree,
		getBlobContent,
		getWorkingChangesDiffs
	} from '$lib/bindings/commands';
	import DiffViewer from './DiffViewer.svelte';
	import FileTree from './FileTree.svelte';
	import BlamePanel from './BlamePanel.svelte';
	import ResizeHandle from './ResizeHandle.svelte';
	import { getClampedLayout, updateLayout } from '$lib/stores/layout';
	import ContextMenu from './ContextMenu.svelte';
	import type { ContextMenuItem } from './ContextMenu.svelte';
	import { SvelteMap } from 'svelte/reactivity';

	interface Props {
		details: CommitDetails;
		repoPath: string;
		onhistoryfile?: (_path: string) => void;
	}

	let { details, repoPath, onhistoryfile }: Props = $props();

	let activeTab = $state<'patch' | 'tree'>('patch');
	let fileTree = $state<FileTreeNode | null>(null);
	let loadingTree = $state(false);
	let blameFilePath = $state<string | null>(null);
	let rightPanelWidth = $state(getClampedLayout().rightPanelWidth);
	let diffMode = $state<'normal' | 'word-diff' | 'stat-only'>('normal');
	let whitespaceMode = $state<
		'none' | 'ignore-space-change' | 'ignore-all-space' | 'ignore-blank-lines'
	>('none');
	let viewMode = $state<'unified' | 'side-by-side'>('unified');

	let fileDiffs = new SvelteMap<string, FileDiff>();
	let diffsLoading = $state(false);
	let blobContent = $state<string | null>(null);
	let blobLoading = $state(false);
	let blobPath = $state<string | null>(null);

	let scrollContainer: HTMLDivElement | undefined = $state();

	function persistRightPanelWidth() {
		updateLayout({ rightPanelWidth });
	}

	let fileContextMenu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null);
	let treeSearchQuery = $state('');

	function handleFileContextMenu(e: MouseEvent, path: string) {
		const items: ContextMenuItem[] = [
			{ label: 'Copy file path', action: () => navigator.clipboard.writeText(path) },
			{ separator: true },
			{ label: 'View file history', shortcut: 'h', action: () => onhistoryfile?.(path) },
			{
				label: 'View blame',
				action: () => {
					blameFilePath = path;
				}
			}
		];
		fileContextMenu = { x: e.clientX, y: e.clientY, items };
	}

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

	$effect(() => {
		void details.info.oid;
		activeTab = 'patch';
		fileTree = null;
		blameFilePath = null;
		blobContent = null;
		blobPath = null;
		fileDiffs.clear();
		loadAllDiffs();
	});

	async function loadAllDiffs() {
		if (details.changed_files.length === 0) return;
		diffsLoading = true;

		if (details.info.oid === '__staged__' || details.info.oid === '__unstaged__') {
			try {
				const diffs = await getWorkingChangesDiffs(
					repoPath,
					details.info.oid === '__staged__',
					diffMode,
					whitespaceMode
				);
				const map = new SvelteMap<string, FileDiff>();
				for (const diff of diffs) {
					map.set(diff.path, diff);
				}
				fileDiffs.clear();
				for (const [k, v] of map) fileDiffs.set(k, v);
			} catch {
				fileDiffs.clear();
			}
			diffsLoading = false;
			return;
		}

		const parentOid = details.info.parent_oids[0] ?? null;
		const promises = details.changed_files.map(async (file) => {
			try {
				const diff = await getFileDiff(
					repoPath,
					parentOid,
					details.info.oid,
					file.path,
					diffMode,
					whitespaceMode
				);
				return [file.path, diff] as const;
			} catch {
				return [file.path, null] as const;
			}
		});
		const results = await Promise.all(promises);
		const map = new SvelteMap<string, FileDiff>();
		for (const [path, diff] of results) {
			if (diff) map.set(path, diff);
		}
		fileDiffs.clear();
		for (const [k, v] of map) fileDiffs.set(k, v);
		diffsLoading = false;
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

	function switchTab(tab: 'patch' | 'tree') {
		activeTab = tab;
		blobContent = null;
		blobPath = null;
		if (tab === 'tree') loadFileTree();
	}

	function scrollToId(id: string) {
		scrollContainer?.querySelector(`#${CSS.escape(id)}`)?.scrollIntoView({ behavior: 'smooth' });
	}

	function scrollToComments() {
		scrollContainer?.scrollTo({ top: 0, behavior: 'smooth' });
	}

	async function showBlob(path: string) {
		if (blobPath === path && blobContent !== null) return;
		blobPath = path;
		blobLoading = true;
		try {
			blobContent = await getBlobContent(repoPath, details.info.oid, path);
		} catch {
			blobContent = null;
		} finally {
			blobLoading = false;
		}
	}

	function openBlame(path: string) {
		blameFilePath = path;
	}

	function formatParent(oid: string): string {
		return oid.substring(0, 7);
	}

	function formatTime(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	function fileHeaderId(index: number): string {
		return `diff-${index}`;
	}
</script>

<div class="flex h-full">
	<div
		class="flex-1 flex flex-col overflow-hidden bg-gray-900"
		role="region"
		aria-label="Diff viewer"
	>
		<div
			class="flex items-center gap-2 border-b border-gray-700 px-4 py-1.5 shrink-0"
			role="toolbar"
			aria-label="Diff controls"
		>
			<button
				class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700"
				onclick={() => (viewMode = viewMode === 'unified' ? 'side-by-side' : 'unified')}
				aria-label="Toggle diff view mode: {viewMode === 'unified' ? 'unified' : 'side by side'}"
			>
				{viewMode === 'unified' ? 'Unified' : 'Side by Side'}
			</button>
			<select
				class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300"
				aria-label="Diff mode"
				bind:value={diffMode}
				onchange={loadAllDiffs}
			>
				<option value="normal">Normal</option>
				<option value="word-diff">Word Diff</option>
				<option value="stat-only">Stat Only</option>
			</select>
			{#if diffMode !== 'stat-only'}
				<select
					class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300"
					aria-label="Whitespace mode"
					bind:value={whitespaceMode}
					onchange={loadAllDiffs}
				>
					<option value="none">Show Whitespace</option>
					<option value="ignore-space-change">Ignore Space Change</option>
					<option value="ignore-all-space">Ignore All Space</option>
					<option value="ignore-blank-lines">Ignore Blank Lines</option>
				</select>
			{/if}
			{#if activeTab === 'patch'}
				<span class="ml-auto text-xs text-gray-500" role="status">
					{details.changed_files.length} file{details.changed_files.length !== 1 ? 's' : ''} changed
				</span>
			{/if}
		</div>

		{#if activeTab === 'tree' && blobPath}
			<div class="flex items-center gap-2 border-b border-gray-700 px-4 py-1 shrink-0">
				<h3 class="font-mono text-sm text-gray-300">{blobPath}</h3>
				<button
					class="ml-auto rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 hover:bg-gray-700"
					aria-label="Open blame view for {blobPath}"
					onclick={() => openBlame(blobPath!)}
				>
					Blame
				</button>
			</div>
		{/if}

		<div class="flex-1 overflow-y-auto" bind:this={scrollContainer}>
			{#if activeTab === 'tree' && blobLoading}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					Loading file content...
				</div>
			{:else if activeTab === 'tree' && blobContent !== null}
				<pre class="p-4 font-mono text-xs text-gray-300 whitespace-pre-wrap">{blobContent}</pre>
			{:else if activeTab === 'tree' && blobPath && blobContent === null}
				<div class="flex items-center justify-center py-8 text-sm text-gray-500">
					Binary file (not displayed)
				</div>
			{:else}
				{#if details.info.oid === '__staged__' || details.info.oid === '__unstaged__'}
					<div class="px-4 py-3 border-b border-gray-800">
						<div class="flex items-center gap-2 text-sm">
							<span
								class="inline-block h-2 w-2 rounded-full {details.info.oid === '__staged__'
									? 'bg-green-400'
									: 'bg-orange-400'}"
							></span>
							<span
								class="font-medium {details.info.oid === '__staged__'
									? 'text-green-300'
									: 'text-orange-300'}"
							>
								{details.info.summary}
							</span>
							<span class="text-xs text-gray-500">
								{details.changed_files.length} file{details.changed_files.length !== 1 ? 's' : ''} changed
							</span>
						</div>
					</div>
				{:else}
					<div class="px-4 py-3 border-b border-gray-800">
						<div class="flex items-baseline gap-3 text-sm">
							<span class="font-mono text-gray-500">commit {details.info.oid.substring(0, 7)}</span>
							{#each details.info.refs as r (r.Branch?.name ?? r.Tag?.name ?? r.Remote?.name ?? '')}
								{#if r.Branch?.is_head}
									<span class="rounded bg-green-700/50 px-1.5 py-0.5 text-xs text-green-300">
										{r.Branch.name}
									</span>
								{/if}
								{#if r.Tag}
									<span class="rounded bg-yellow-700/50 px-1.5 py-0.5 text-xs text-yellow-300">
										{r.Tag.name}
									</span>
								{/if}
							{/each}
						</div>
						<div class="mt-1 text-xs text-gray-400">
							Author: {details.info.author.name}
							<span class="text-gray-600">&lt;{details.info.author.email}&gt;</span>
						</div>
						<div class="text-xs text-gray-400">
							Date: {formatTime(details.info.author_time)}
						</div>
						{#if details.info.committer.name !== details.info.author.name}
							<div class="text-xs text-gray-400">
								Committer: {details.info.committer.name}
								<span class="text-gray-600">&lt;{details.info.committer.email}&gt;</span>
							</div>
						{/if}
						{#if details.info.parent_oids.length > 0}
							<div class="text-xs text-gray-500">
								Parent{details.info.parent_oids.length > 1 ? 's' : ''}:
								{#each details.info.parent_oids as p, i (i)}
									<span class="font-mono">{formatParent(p)}</span>{i <
									details.info.parent_oids.length - 1
										? ', '
										: ''}
								{/each}
							</div>
						{/if}
						<div class="mt-2 text-sm text-gray-200 whitespace-pre-wrap">{details.info.summary}</div>
						{#if details.body}
							<pre class="mt-1 text-sm text-gray-400 whitespace-pre-wrap">{details.body}</pre>
						{/if}
					</div>
				{/if}

				{#if diffsLoading}
					<div class="flex items-center justify-center py-4 text-sm text-gray-500">
						Loading diffs...
					</div>
				{:else if details.changed_files.length === 0}
					<div class="flex items-center justify-center py-4 text-sm text-gray-500">
						No changed files
					</div>
				{:else}
					{#each details.changed_files as file, i (file.path)}
						{@const diff = fileDiffs.get(file.path)}
						<div id={fileHeaderId(i)} class="border-b border-gray-800">
							<div class="flex items-center gap-2 bg-gray-800/60 px-4 py-1.5 sticky top-0 z-10">
								<span class="font-bold {CHANGE_COLORS[file.change_type] ?? ''}">
									{CHANGE_LETTERS[file.change_type] ?? '?'}
								</span>
								<span class="font-mono text-xs text-gray-300">{file.path}</span>
								{#if !file.is_binary && !file.is_submodule}
									<span class="ml-1 font-mono text-[10px]">
										<span class="text-green-500"
											>{file.additions > 0 ? '+' + file.additions : ''}</span
										>
										<span class="text-red-500"
											>{file.deletions > 0 ? '-' + file.deletions : ''}</span
										>
									</span>
								{:else if file.is_binary}
									<span class="text-[10px] text-gray-500">binary</span>
								{:else if file.is_submodule}
									<span class="text-[10px] text-orange-400">submodule</span>
								{/if}
								<button
									class="ml-auto rounded border border-gray-700 bg-gray-800 px-2 py-0.5 text-[10px] text-gray-300 hover:bg-gray-700"
									aria-label="Open blame view for {file.path}"
									onclick={() => openBlame(file.path)}
								>
									Blame
								</button>
							</div>
							{#if diff}
								{#if diff.is_submodule}
									<div class="px-4 py-3 text-xs text-orange-400">
										{diff.hunks
											.flatMap((h) => h.lines)
											.map((l) => ('Addition' in l ? l.Addition.content : ''))
											.filter(Boolean)
											.join(' ')}
									</div>
								{:else if diff.is_binary}
									<div class="px-4 py-3 text-xs text-gray-500">Binary file (not displayed)</div>
								{:else if diff.hunks.length > 0}
									<div class="p-2">
										<DiffViewer hunks={diff.hunks} {viewMode} />
									</div>
								{:else}
									<div class="px-4 py-3 text-xs text-gray-500">No content changes</div>
								{/if}
								{#if diff.truncated_at != null}
									<div
										class="flex items-center justify-center gap-3 border-t border-gray-700 py-1.5"
									>
										<span class="text-xs text-gray-500">
											Truncated at {diff.truncated_at} lines
										</span>
									</div>
								{/if}
							{:else if file.is_binary || file.is_submodule}
								<div class="px-4 py-3 text-xs text-gray-500">
									{file.is_submodule ? 'Submodule reference' : 'Binary file (not displayed)'}
								</div>
							{:else}
								<div class="px-4 py-3 text-xs text-gray-500">Loading...</div>
							{/if}
						</div>
					{/each}
				{/if}
			{/if}
		</div>

		{#if blameFilePath}
			<div class="absolute inset-0 z-20 bg-gray-900">
				<BlamePanel
					{repoPath}
					filePath={blameFilePath}
					atCommit={details.info.oid}
					onclose={() => (blameFilePath = null)}
				/>
			</div>
		{/if}
	</div>

	<ResizeHandle
		direction="horizontal"
		bind:panelWidth={rightPanelWidth}
		onDragEnd={persistRightPanelWidth}
	/>

	<div
		class="shrink-0 flex flex-col border-l border-gray-700 bg-gray-900/50 overflow-hidden"
		style="width: {rightPanelWidth}px;"
		role="region"
		aria-label="File list"
	>
		<div class="flex border-b border-gray-700 shrink-0" role="tablist" aria-label="File list tabs">
			<button
				role="tab"
				aria-selected={activeTab === 'patch'}
				aria-controls="file-list-panel"
				class="flex-1 px-3 py-2 text-xs font-semibold uppercase tracking-wider transition-colors {activeTab ===
				'patch'
					? 'text-gray-200 border-b-2 border-blue-500'
					: 'text-gray-500 hover:text-gray-300'}"
				onclick={() => switchTab('patch')}
			>
				Patch
			</button>
			<button
				role="tab"
				aria-selected={activeTab === 'tree'}
				aria-controls="file-list-panel"
				class="flex-1 px-3 py-2 text-xs font-semibold uppercase tracking-wider transition-colors {activeTab ===
				'tree'
					? 'text-gray-200 border-b-2 border-blue-500'
					: 'text-gray-500 hover:text-gray-300'}"
				onclick={() => switchTab('tree')}
			>
				Tree
			</button>
		</div>

		<div id="file-list-panel" role="tabpanel" class="flex-1 overflow-y-auto">
			{#if activeTab === 'patch'}
				<button
					class="flex w-full items-center gap-2 border-b border-gray-800 px-3 py-1.5 text-left text-xs hover:bg-gray-800/70"
					aria-label="Scroll to commit comments"
					onclick={scrollToComments}
				>
					<span class="flex-1 text-gray-400">Comments</span>
				</button>
				{#each details.changed_files as file, i (file.path)}
					<button
						class="flex w-full items-center gap-2 border-b border-gray-800 px-3 py-1.5 text-left text-xs hover:bg-gray-800/70"
						aria-label="{file.path}, {CHANGE_LETTERS[file.change_type] ??
							'?'}, {file.additions} additions, {file.deletions} deletions"
						onclick={() => scrollToId(fileHeaderId(i))}
					>
						<span class="w-4 text-center font-bold {CHANGE_COLORS[file.change_type] ?? ''}">
							{CHANGE_LETTERS[file.change_type] ?? '?'}
						</span>
						<span class="flex-1 truncate font-mono text-gray-300">{file.path}</span>
						{#if !file.is_binary && !file.is_submodule}
							<span class="shrink-0 font-mono text-[10px]">
								<span class="text-green-500">{file.additions > 0 ? '+' + file.additions : ''}</span>
								<span class="text-red-500">{file.deletions > 0 ? '-' + file.deletions : ''}</span>
							</span>
						{:else if file.is_binary}
							<span class="text-[10px] text-gray-500">bin</span>
						{:else if file.is_submodule}
							<span class="text-[10px] text-orange-400">sub</span>
						{/if}
					</button>
				{/each}
			{:else if loadingTree}
				<div class="px-3 py-4 text-xs text-gray-500">Loading tree...</div>
			{:else if fileTree}
				<div class="border-b border-gray-800 px-2 py-1">
					<input
						type="text"
						class="w-full rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500 outline-none focus:border-blue-500"
						placeholder="Search files..."
						bind:value={treeSearchQuery}
						aria-label="Search file tree"
					/>
				</div>
				<FileTree
					node={fileTree}
					{repoPath}
					onhistoryfile={(p: string) => onhistoryfile?.(p)}
					onselectfile={(p: string) => showBlob(p)}
					onfilecontextmenu={handleFileContextMenu}
					filter={treeSearchQuery}
				/>
			{:else}
				<div class="px-3 py-4 text-xs text-gray-500">No file tree</div>
			{/if}
		</div>
	</div>
</div>

{#if fileContextMenu}
	<ContextMenu
		x={fileContextMenu.x}
		y={fileContextMenu.y}
		items={fileContextMenu.items}
		onclose={() => (fileContextMenu = null)}
	/>
{/if}
