<script lang="ts">
	import { onMount } from 'svelte';
	import {
		openRepository,
		getCommits,
		getGraphLayout,
		getCommitDetails,
		getRefs,
		getWorkingChanges
	} from '$lib/bindings/commands';
	import type {
		CommitInfo,
		GraphLayout,
		CommitDetails,
		Ref,
		WorkingChangesDiff,
		FileChange
	} from '$lib/bindings/types';
	import {
		repoInfo,
		selectedOid,
		error,
		matchingOids,
		comparisonOid,
		graphColorMode,
		graphHideMerges,
		graphOrientation,
		graphPalette,
		operationState
	} from '$lib/stores/repository';
	import CommitList from '$lib/components/CommitList.svelte';
	import SearchBar from '$lib/components/SearchBar.svelte';
	import CommitDetailPanel from '$lib/components/CommitDetailPanel.svelte';
	import ComparisonPanel from '$lib/components/ComparisonPanel.svelte';
	import ResizeHandle from '$lib/components/ResizeHandle.svelte';
	import Sidebar from '$lib/components/Sidebar/Sidebar.svelte';
	import RefList from '$lib/components/Sidebar/RefList.svelte';
	import StashList from '$lib/components/Sidebar/StashList.svelte';
	import ReflogPanel from '$lib/components/Sidebar/ReflogPanel.svelte';
	import FileHistoryPanel from '$lib/components/FileHistoryPanel.svelte';
	import Toolbar from '$lib/components/Toolbar.svelte';
	import AuthorLegend from '$lib/components/AuthorLegend.svelte';
	import ToastContainer from '$lib/components/ToastContainer.svelte';
	import { showToast } from '$lib/stores/toast';
	import {
		toggleDebug,
		tickFps,
		updateDebugGraphStats,
		updateDebugCommitCounts
	} from '$lib/stores/debug';
	import DebugOverlay from '$lib/components/DebugOverlay.svelte';
	import { getClampedLayout, updateLayout } from '$lib/stores/layout';
	import { registerCommand } from '$lib/stores/commands';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import type { ContextMenuItem } from '$lib/components/ContextMenu.svelte';

	let repoPath = $state('');
	let commits = $state<CommitInfo[]>([]);
	let graphLayout = $state<GraphLayout | null>(null);
	let commitDetails = $state<CommitDetails | null>(null);
	let detailsLoading = $state(false);
	let savedLayout = typeof window !== 'undefined' ? getClampedLayout() : null;
	let detailPanelHeight = $state(savedLayout?.detailPanelHeight ?? 400);
	let viewportHeight = $state(typeof window !== 'undefined' ? window.innerHeight : 800);
	let allRefs = $state<Ref[]>([]);
	let historyFilePath = $state<string | null>(null);
	let historyRevision = $state(0);
	let sidebarGotoTab = $state<'refs' | 'stash' | 'reflog' | 'history'>('refs');
	let workingChangesDiff = $state<WorkingChangesDiff | null>(null);
	let showCommandPalette = $state(false);
	let contextMenu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null);
	let isDragging = $state(false);
	let isFullscreen = $state(false);

	let sidebarWidth = $state(savedLayout?.sidebarWidth ?? 220);

	const STAGED_OID = '__staged__';
	const UNSTAGED_OID = '__unstaged__';
	const VIRTUAL_OIDS = new Set([STAGED_OID, UNSTAGED_OID]);

	onMount(() => {
		registerCommands();
		const params = new URLSearchParams(window.location.search);
		const path = params.get('path');
		if (path) {
			repoPath = path;
			loadRepo(path);
		}

		function onResize() {
			viewportHeight = window.innerHeight;
		}
		window.addEventListener('resize', onResize);
		window.addEventListener('keydown', handleKeydown);

		let fpsRafId = 0;
		function fpsLoop() {
			tickFps();
			fpsRafId = requestAnimationFrame(fpsLoop);
		}
		fpsRafId = requestAnimationFrame(fpsLoop);

		return () => {
			window.removeEventListener('resize', onResize);
			window.removeEventListener('keydown', handleKeydown);
			cancelAnimationFrame(fpsRafId);
		};
	});

	let loadError = $state<string | null>(null);

	async function loadRepo(path: string) {
		operationState.set('LoadingRepo');
		error.set(null);
		loadError = null;
		try {
			const info = await openRepository(path);
			repoInfo.set(info);
		} catch (e: unknown) {
			const msg = e instanceof Error ? e.message : String(e);
			error.set(msg);
			showToast(`Failed to open repository: ${msg}`, 'error');
			operationState.set('Idle');
			return;
		}

		let commitCount = 0;
		try {
			const loadedCommits = await getCommits(path);
			commits = loadedCommits;
			commitCount = loadedCommits.length;
		} catch (e: unknown) {
			loadError = e instanceof Error ? e.message : String(e);
			showToast('Partial load: commits failed', 'warning');
		}

		try {
			const layout = await getGraphLayout(path, {
				hide_merges: $graphHideMerges,
				orientation: $graphOrientation,
				color_mode: $graphColorMode,
				palette: $graphPalette
			});
			graphLayout = layout;
			layoutLoaded = true;
		} catch (e: unknown) {
			loadError = loadError ?? (e instanceof Error ? e.message : String(e));
			showToast('Partial load: graph failed', 'warning');
		}

		try {
			allRefs = await getRefs(path);
		} catch (e: unknown) {
			loadError = loadError ?? (e instanceof Error ? e.message : String(e));
		}

		try {
			workingChangesDiff = await getWorkingChanges(path);
		} catch {
			workingChangesDiff = null;
		}

		if (!loadError) {
			showToast(`${commitCount} commits loaded`, 'info');
		}
		operationState.set('Idle');
	}

	function handleOpen() {
		if (repoPath.trim()) {
			loadRepo(repoPath.trim());
		}
	}

	async function reloadLayout() {
		if (!repoPath) return;
		operationState.set('ApplyingFilter');
		try {
			graphLayout = await getGraphLayout(repoPath, {
				hide_merges: $graphHideMerges,
				orientation: $graphOrientation,
				color_mode: $graphColorMode,
				palette: $graphPalette
			});
		} catch {
			// keep existing layout
		} finally {
			if ($operationState === 'ApplyingFilter') operationState.set('Idle');
		}
	}

	let layoutLoaded = $state(false);

	function makeVirtualCommit(oid: string, summary: string, _fileCount: number): CommitInfo {
		return {
			oid,
			short_oid: '',
			message: summary,
			summary,
			author: { name: '', email: '' },
			committer: { name: '', email: '' },
			author_time: '',
			commit_time: '',
			parent_oids: [],
			refs: []
		};
	}

	let displayCommits = $derived.by(() => {
		if (!workingChangesDiff) return commits;
		const hasStaged = workingChangesDiff.staged.length > 0;
		const hasUnstaged = workingChangesDiff.unstaged.length > 0;
		if (!hasStaged && !hasUnstaged) return commits;
		const virtuals: CommitInfo[] = [];
		if (hasUnstaged)
			virtuals.push(
				makeVirtualCommit(UNSTAGED_OID, 'Unstaged changes', workingChangesDiff.unstaged.length)
			);
		if (hasStaged)
			virtuals.push(
				makeVirtualCommit(STAGED_OID, 'Staged changes', workingChangesDiff.staged.length)
			);
		return [...virtuals, ...commits];
	});

	let displayLayout = $derived.by(() => {
		if (!graphLayout) return null;
		if (!workingChangesDiff) return graphLayout;
		const hasStaged = workingChangesDiff.staged.length > 0;
		const hasUnstaged = workingChangesDiff.unstaged.length > 0;
		if (!hasStaged && !hasUnstaged) return graphLayout;
		const virtualCount = (hasStaged ? 1 : 0) + (hasUnstaged ? 1 : 0);
		const virtualNodes: import('$lib/bindings/types').NodePosition[] = [];
		if (hasUnstaged) {
			virtualNodes.push({
				oid: UNSTAGED_OID,
				row: 0,
				column: 0,
				is_merge: false,
				color: { r: 255, g: 255, b: 255, a: 255 },
				is_dimmed: false,
				is_highlighted: false
			});
		}
		if (hasStaged) {
			virtualNodes.push({
				oid: STAGED_OID,
				row: hasUnstaged ? 1 : 0,
				column: 0,
				is_merge: false,
				color: { r: 255, g: 255, b: 255, a: 255 },
				is_dimmed: false,
				is_highlighted: false
			});
		}
		return {
			...graphLayout,
			nodes: [
				...virtualNodes,
				...graphLayout.nodes.map((n) => ({ ...n, row: n.row + virtualCount }))
			],
			edges: graphLayout.edges.map((e) => ({
				...e,
				from_row: e.from_row + virtualCount,
				to_row: e.to_row + virtualCount
			})),
			stash_markers: graphLayout.stash_markers.map((s) => ({ ...s, row: s.row + virtualCount })),
			total_rows: graphLayout.total_rows + virtualCount
		};
	});

	$effect(() => {
		void $graphColorMode;
		void $graphHideMerges;
		void $graphOrientation;
		void $graphPalette;
		if ($repoInfo && layoutLoaded) reloadLayout();
	});

	$effect(() => {
		if (graphLayout) {
			updateDebugGraphStats(
				graphLayout.nodes.length,
				graphLayout.edges.length,
				graphLayout.stash_markers.length,
				graphLayout.total_columns
			);
		}
	});

	async function onSelectCommit(oid: string, ctrlKey = false) {
		if (ctrlKey && $selectedOid && $selectedOid !== oid) {
			comparisonOid.set(oid);
			return;
		}
		comparisonOid.set(null);
		selectedOid.set(oid);
		commitDetails = null;
		detailsLoading = true;
		operationState.set('LoadingDetails');

		if (VIRTUAL_OIDS.has(oid)) {
			const files: FileChange[] =
				oid === STAGED_OID
					? (workingChangesDiff?.staged ?? [])
					: (workingChangesDiff?.unstaged ?? []);
			const label = oid === STAGED_OID ? 'Staged changes' : 'Unstaged changes';
			commitDetails = {
				info: {
					oid,
					short_oid: '',
					message: label,
					summary: label,
					author: { name: '', email: '' },
					committer: { name: '', email: '' },
					author_time: '',
					commit_time: '',
					parent_oids: [],
					refs: []
				},
				tree_oid: '',
				signature: null,
				changed_files: files,
				body: null
			};
			detailsLoading = false;
			if ($operationState === 'LoadingDetails') operationState.set('Idle');
			return;
		}

		try {
			commitDetails = await getCommitDetails(repoPath, oid);
		} catch {
			commitDetails = null;
		} finally {
			detailsLoading = false;
			if ($operationState === 'LoadingDetails') operationState.set('Idle');
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'F12' || (e.key === 'D' && e.ctrlKey && e.shiftKey)) {
			e.preventDefault();
			toggleDebug();
			return;
		}
		if ((e.key === 'p' && e.ctrlKey) || (e.key === 'p' && e.metaKey)) {
			e.preventDefault();
			showCommandPalette = true;
			return;
		}
		if (e.key === 'm' && e.ctrlKey) {
			e.preventDefault();
			isFullscreen = !isFullscreen;
			return;
		}
		if (e.key === 'Escape') {
			if (showCommandPalette || contextMenu) return;
			if (isFullscreen) {
				isFullscreen = false;
				return;
			}
			comparisonOid.set(null);
			return;
		}
		if (!$selectedOid || !commits.length) return;

		const currentCommit = commits.find((c) => c.oid === $selectedOid);
		if (!currentCommit) return;

		if (e.key === 'n' && e.altKey) {
			e.preventDefault();
			navigateAuthor(currentCommit, 1);
		} else if (e.key === 'p' && e.altKey) {
			e.preventDefault();
			navigateAuthor(currentCommit, -1);
		}
	}

	function navigateAuthor(current: CommitInfo, direction: 1 | -1) {
		const authorKey = current.author.name + ' <' + current.author.email + '>';
		const idx = commits.findIndex((c) => c.oid === current.oid);
		if (idx < 0) return;

		for (let i = idx + direction; i >= 0 && i < commits.length; i += direction) {
			const c = commits[i];
			const key = c.author.name + ' <' + c.author.email + '>';
			if (key === authorKey) {
				onSelectCommit(c.oid);
				return;
			}
		}
	}

	function persistDetailPanelHeight() {
		updateLayout({ detailPanelHeight });
	}

	function registerCommands() {
		registerCommand({
			id: 'toggle-merges',
			label: 'Toggle merge commits',
			shortcut: undefined,
			category: 'Graph',
			action: () => graphHideMerges.update((v) => !v)
		});
		registerCommand({
			id: 'toggle-color-mode',
			label: 'Toggle color by author',
			shortcut: undefined,
			category: 'Graph',
			action: () => graphColorMode.update((v) => (v === 'by-branch' ? 'by-author' : 'by-branch'))
		});
		registerCommand({
			id: 'toggle-orientation',
			label: 'Toggle graph orientation',
			shortcut: undefined,
			category: 'Graph',
			action: () =>
				graphOrientation.update((v) => (v === 'top-to-bottom' ? 'bottom-to-top' : 'top-to-bottom'))
		});
		registerCommand({
			id: 'toggle-debug',
			label: 'Toggle debug overlay',
			shortcut: 'F12',
			category: 'Debug',
			action: toggleDebug
		});
		registerCommand({
			id: 'palette-default',
			label: 'Graph palette: Default',
			category: 'Palette',
			action: () => graphPalette.set('default')
		});
		registerCommand({
			id: 'palette-deuteranopia',
			label: 'Graph palette: Deuteranopia-safe',
			category: 'Palette',
			action: () => graphPalette.set('deuteranopia')
		});
		registerCommand({
			id: 'palette-protanopia',
			label: 'Graph palette: Protanopia-safe',
			category: 'Palette',
			action: () => graphPalette.set('protanopia')
		});
		registerCommand({
			id: 'palette-tritanopia',
			label: 'Graph palette: Tritanopia-safe',
			category: 'Palette',
			action: () => graphPalette.set('tritanopia')
		});
		registerCommand({
			id: 'toggle-fullscreen',
			label: 'Toggle fullscreen mode',
			shortcut: 'Ctrl+M',
			category: 'View',
			action: () => {
				isFullscreen = !isFullscreen;
			}
		});
	}

	function handleCommitContextMenu(e: MouseEvent, oid: string) {
		e.preventDefault();
		const commit = commits.find((c) => c.oid === oid);
		const items: ContextMenuItem[] = [
			{ label: 'Copy SHA', action: () => navigator.clipboard.writeText(oid) },
			{ label: 'Copy short SHA', action: () => navigator.clipboard.writeText(oid.substring(0, 7)) }
		];
		if (commit) {
			items.push({
				label: 'Copy commit message',
				action: () => navigator.clipboard.writeText(commit.message)
			});
			items.push({ separator: true });
			if ($selectedOid && $selectedOid !== oid) {
				items.push({ label: 'Compare with selected', action: () => comparisonOid.set(oid) });
			}
		}
		contextMenu = { x: e.clientX, y: e.clientY, items };
	}

	function handleBranchContextMenu(e: MouseEvent, name: string) {
		const items: ContextMenuItem[] = [
			{ label: 'Copy branch name', action: () => navigator.clipboard.writeText(name) }
		];
		contextMenu = { x: e.clientX, y: e.clientY, items };
	}

	function handleDragOver(e: DragEvent) {
		if (e.dataTransfer?.types.includes('Files')) {
			e.preventDefault();
			e.dataTransfer.dropEffect = 'move';
			isDragging = true;
		}
	}

	function handleDragLeave() {
		isDragging = false;
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
		const files = e.dataTransfer?.files;
		if (!files || files.length === 0) return;
		const file = files[0] as File & { path?: string };
		if (file.path) {
			repoPath = file.path;
			loadRepo(file.path);
		}
	}
</script>

<div
	class="flex h-screen flex-col bg-gray-900 text-gray-100 {isDragging
		? 'ring-2 ring-blue-500 ring-inset'
		: ''}"
	ondragover={handleDragOver}
	ondragleave={handleDragLeave}
	ondrop={handleDrop}
>
	{#if !$repoInfo}
		<div class="flex flex-1 items-center justify-center" role="main">
			<div class="w-full max-w-md space-y-4 p-8">
				<h1 class="text-2xl font-bold">gitv</h1>
				<p class="text-gray-400">Modern Git repository visualizer</p>
				<div class="flex gap-2">
					<input
						type="text"
						class="flex-1 rounded border border-gray-700 bg-gray-800 px-3 py-2 text-sm"
						placeholder="/path/to/repository"
						aria-label="Repository path"
						bind:value={repoPath}
						onkeydown={(e) => e.key === 'Enter' && handleOpen()}
					/>
					<button
						class="rounded bg-blue-600 px-4 py-2 text-sm hover:bg-blue-700"
						onclick={handleOpen}
						aria-label="Open repository"
					>
						Open
					</button>
				</div>
				{#if $error}
					<p class="text-sm text-red-400" role="alert">{$error}</p>
				{/if}
			</div>
		</div>
	{:else}
		{#if !isFullscreen}
			<header class="flex items-center gap-3 border-b border-gray-800 px-4 py-2">
				<span class="font-mono text-sm text-gray-400">{$repoInfo.path}</span>
				{#if $repoInfo.head_branch}
					<span class="rounded bg-green-700/50 px-2 py-0.5 text-xs text-green-300">
						{$repoInfo.head_branch}
					</span>
				{/if}
				{#if $repoInfo.is_bare}
					<span class="rounded bg-gray-700/50 px-2 py-0.5 text-xs text-gray-400">
						bare repository
					</span>
				{/if}
				{#if $graphHideMerges}
					<span class="rounded bg-yellow-700/50 px-2 py-0.5 text-xs text-yellow-300">
						Merges hidden
					</span>
				{/if}
				<Toolbar />
				{#if graphLayout}
					<AuthorLegend layout={graphLayout} />
				{/if}
				<div class="ml-auto w-80">
					<SearchBar {repoPath} />
				</div>
				{#if loadError}
					<span class="flex items-center gap-2 text-xs text-amber-400">
						Loading incomplete
						<button
							class="rounded bg-amber-700/50 px-2 py-0.5 text-xs hover:bg-amber-700"
							onclick={() => loadRepo(repoPath)}
							aria-label="Retry loading repository"
						>
							Retry
						</button>
					</span>
				{/if}
				{#if $operationState === 'LoadingRepo'}
					<span class="text-xs text-gray-500" role="status" aria-live="polite"
						>Loading repository...</span
					>
				{:else if $operationState === 'LoadingDetails'}
					<span class="text-xs text-gray-500" role="status" aria-live="polite"
						>Loading details...</span
					>
				{:else if $operationState === 'Searching'}
					<span class="text-xs text-gray-500" role="status" aria-live="polite">Searching...</span>
				{:else if $operationState === 'ApplyingFilter'}
					<span class="text-xs text-gray-500" role="status" aria-live="polite"
						>Applying filter...</span
					>
				{/if}
			</header>
		{/if}
		<div class="flex flex-1 overflow-hidden">
			{#if !isFullscreen}
				<Sidebar gotoTab={sidebarGotoTab} width={sidebarWidth}>
					{#snippet refs()}
						<RefList refs={allRefs} onbranchcontextmenu={handleBranchContextMenu} />
					{/snippet}
					{#snippet stash()}
						<StashList {repoPath} />
					{/snippet}
					{#snippet reflog()}
						<ReflogPanel {repoPath} onentryselect={(oid) => onSelectCommit(oid)} />
					{/snippet}
					{#snippet history()}
						{#if historyFilePath}
							<FileHistoryPanel
								{repoPath}
								filePath={historyFilePath}
								revision={historyRevision}
								onenterselect={(oid: string) => onSelectCommit(oid)}
							/>
						{:else}
							<div class="text-gray-500 italic">No file selected</div>
						{/if}
					{/snippet}
				</Sidebar>
			{/if}
			<div class="flex-1 overflow-hidden flex flex-col" role="main" aria-label="Commit history">
				<div class="flex-1 overflow-hidden">
					{#if displayLayout}
						<CommitList
							commits={displayCommits}
							layout={displayLayout}
							selectedOid={$selectedOid}
							matchingOids={$matchingOids}
							onSelect={(oid: string, ctrlKey: boolean) => onSelectCommit(oid, ctrlKey)}
							onContextMenu={handleCommitContextMenu}
							graphWidth={savedLayout?.graphWidth ?? 200}
						/>
					{/if}
				</div>

				{#if $selectedOid}
					<ResizeHandle
						bind:panelHeight={detailPanelHeight}
						maxHeight={Math.floor(viewportHeight * 0.9)}
						onDragEnd={persistDetailPanelHeight}
					/>
					<div
						class="overflow-hidden bg-gray-900 border-t border-gray-700"
						style="height: {detailPanelHeight}px;"
						role="region"
						aria-label="Commit details"
					>
						{#if $comparisonOid}
							<ComparisonPanel {repoPath} fromOid={$selectedOid} toOid={$comparisonOid} />
						{:else if detailsLoading}
							<div
								class="flex items-center justify-center h-full text-sm text-gray-500"
								role="status"
								aria-live="polite"
							>
								Loading details...
							</div>
						{:else if commitDetails}
							<CommitDetailPanel
								details={commitDetails}
								{repoPath}
								onhistoryfile={(p: string) => {
									historyFilePath = p;
									historyRevision++;
									sidebarGotoTab = 'history';
								}}
							/>
						{:else}
							<div
								class="flex items-center justify-center h-full text-sm text-gray-500"
								role="alert"
							>
								Failed to load commit details
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
	<ToastContainer />
	<DebugOverlay />
	{#if showCommandPalette}
		<CommandPalette onclose={() => (showCommandPalette = false)} />
	{/if}
	{#if contextMenu}
		<ContextMenu
			x={contextMenu.x}
			y={contextMenu.y}
			items={contextMenu.items}
			onclose={() => (contextMenu = null)}
		/>
	{/if}
	<div class="sr-only" aria-live="polite" role="status" id="a11y-announcer"></div>
</div>
