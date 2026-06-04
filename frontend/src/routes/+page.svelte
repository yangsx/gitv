<script lang="ts">
	import { onMount } from 'svelte';
	import { openRepository, getCommits, getGraphLayout } from '$lib/bindings/commands';
	import type { CommitInfo, GraphLayout } from '$lib/bindings/types';
	import { repoInfo, selectedOid, isLoading, error, matchingOids } from '$lib/stores/repository';
	import CommitList from '$lib/components/CommitList.svelte';
	import SearchBar from '$lib/components/SearchBar.svelte';

	let repoPath = $state('');
	let commits = $state<CommitInfo[]>([]);
	let graphLayout = $state<GraphLayout | null>(null);

	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const path = params.get('path');
		if (path) {
			repoPath = path;
			loadRepo(path);
		}
	});

	async function loadRepo(path: string) {
		isLoading.set(true);
		error.set(null);
		try {
			const [info, loadedCommits, layout] = await Promise.all([
				openRepository(path),
				getCommits(path),
				getGraphLayout(path)
			]);
			repoInfo.set(info);
			commits = loadedCommits;
			graphLayout = layout;
		} catch (e: unknown) {
			error.set(e instanceof Error ? e.message : String(e));
		} finally {
			isLoading.set(false);
		}
	}

	function handleOpen() {
		if (repoPath.trim()) {
			loadRepo(repoPath.trim());
		}
	}

	function onSelectCommit(oid: string) {
		selectedOid.set(oid);
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
			<div class="ml-auto w-80">
				<SearchBar {repoPath} />
			</div>
			{#if $isLoading}
				<span class="text-xs text-gray-500">Loading...</span>
			{/if}
		</header>
		<div class="flex-1 overflow-hidden">
			{#if graphLayout}
				<CommitList
					{commits}
					layout={graphLayout}
					selectedOid={$selectedOid}
					matchingOids={$matchingOids}
					onSelect={onSelectCommit}
				/>
			{/if}
		</div>
	{/if}
</div>
