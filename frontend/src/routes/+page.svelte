<script lang="ts">
	import { onMount } from 'svelte';
	import {
		openRepository,
		getCommits,
		getGraphLayout,
		getCommitDetails,
		getRefs
	} from '$lib/bindings/commands';
	import type { CommitInfo, GraphLayout, CommitDetails, Ref } from '$lib/bindings/types';
	import {
		repoInfo,
		selectedOid,
		error,
		matchingOids,
		comparisonOid,
		graphColorMode,
		graphHideMerges,
		graphOrientation,
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

	let repoPath = $state('');
	let commits = $state<CommitInfo[]>([]);
	let graphLayout = $state<GraphLayout | null>(null);
	let commitDetails = $state<CommitDetails | null>(null);
	let detailsLoading = $state(false);
	let detailPanelHeight = $state(typeof window !== 'undefined' ? Math.floor(window.innerHeight * 0.7) : 400);
	let viewportHeight = $state(typeof window !== 'undefined' ? window.innerHeight : 800);
	let allRefs = $state<Ref[]>([]);
	let historyFilePath = $state<string | null>(null);
	let historyRevision = $state(0);
	let sidebarGotoTab = $state<'refs' | 'stash' | 'reflog' | 'history'>('refs');

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const path = params.get('path');
		if (path) {
			repoPath = path;
			loadRepo(path);
		}

		function onResize() {
			viewportHeight = window.innerHeight;
			detailPanelHeight = Math.floor(window.innerHeight * 0.7);
		}
		window.addEventListener('resize', onResize);
		window.addEventListener('keydown', handleKeydown);
		return () => {
			window.removeEventListener('resize', onResize);
			window.removeEventListener('keydown', handleKeydown);
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
				color_mode: $graphColorMode
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
				color_mode: $graphColorMode
			});
		} catch {
			// keep existing layout
		} finally {
			if ($operationState === 'ApplyingFilter') operationState.set('Idle');
		}
	}

	let layoutLoaded = $state(false);

	$effect(() => {
		void $graphColorMode;
		void $graphHideMerges;
		void $graphOrientation;
		if ($repoInfo && layoutLoaded) reloadLayout();
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
		if (e.key === 'Escape') {
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
</script>

<div class="flex h-screen flex-col bg-gray-900 text-gray-100">
	{#if !$repoInfo}
		<div class="flex flex-1 items-center justify-center">
			<div class="w-full max-w-md space-y-4 p-8">
				<h1 class="text-2xl font-bold">gitv</h1>
				<p class="text-gray-400">Modern Git repository visualizer</p>
				<div class="flex gap-2">
					<input
						type="text"
						class="flex-1 rounded border border-gray-700 bg-gray-800 px-3 py-2 text-sm"
						placeholder="/path/to/repository"
						bind:value={repoPath}
						onkeydown={(e) => e.key === 'Enter' && handleOpen()}
					/>
					<button
						class="rounded bg-blue-600 px-4 py-2 text-sm hover:bg-blue-700"
						onclick={handleOpen}
					>
						Open
					</button>
				</div>
				{#if $error}
					<p class="text-sm text-red-400">{$error}</p>
				{/if}
			</div>
		</div>
	{:else}
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
					>
						Retry
					</button>
				</span>
			{/if}
			{#if $operationState === 'LoadingRepo'}
				<span class="text-xs text-gray-500">Loading repository...</span>
			{:else if $operationState === 'LoadingDetails'}
				<span class="text-xs text-gray-500">Loading details...</span>
			{:else if $operationState === 'Searching'}
				<span class="text-xs text-gray-500">Searching...</span>
			{:else if $operationState === 'ApplyingFilter'}
				<span class="text-xs text-gray-500">Applying filter...</span>
			{/if}
		</header>
		<div class="flex flex-1 overflow-hidden">
			<Sidebar gotoTab={sidebarGotoTab}>
				{#snippet refs()}
					<RefList refs={allRefs} />
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
			<div class="flex-1 overflow-hidden flex flex-col">
				<div class="flex-1 overflow-hidden">
					{#if graphLayout}
						<CommitList
							{commits}
							layout={graphLayout}
							selectedOid={$selectedOid}
							matchingOids={$matchingOids}
							onSelect={(oid: string, ctrlKey: boolean) => onSelectCommit(oid, ctrlKey)}
						/>
					{/if}
				</div>

				{#if $selectedOid}
					<ResizeHandle
						bind:panelHeight={detailPanelHeight}
						maxHeight={Math.floor(viewportHeight * 0.9)}
					/>
					<div
						class="overflow-hidden bg-gray-900 border-t border-gray-700"
						style="height: {detailPanelHeight}px;"
					>
						{#if $comparisonOid}
							<ComparisonPanel {repoPath} fromOid={$selectedOid} toOid={$comparisonOid} />
						{:else if detailsLoading}
							<div class="flex items-center justify-center h-full text-sm text-gray-500">
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
							<div class="flex items-center justify-center h-full text-sm text-gray-500">
								Failed to load commit details
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
	<ToastContainer />
</div>
