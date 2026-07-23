<script lang="ts">
	import { t } from '$lib/stores/locale';
	import { debug, avgIpcTime, recentIpcTimings, formatBytes } from '$lib/stores/debug';
	import { operationState } from '$lib/stores/repository';
	import { runSelfTest } from '$lib/bindings/commands';
	import type { SelfTestResult } from '$lib/bindings/types';

	interface Props {
		repoPath?: string | null;
	}

	let { repoPath = null }: Props = $props();

	$effect(() => {
		void repoPath;
		selfTestResult = null;
		selfTestRunning = false;
		selfTestError = null;
	});

	let formatMs = (ms: number) => ms.toFixed(1);
	let selfTestResult = $state<SelfTestResult | null>(null);
	let selfTestRunning = $state(false);
	let selfTestError = $state<string | null>(null);

	async function runTest() {
		if (!repoPath || selfTestRunning) return;
		selfTestRunning = true;
		selfTestError = null;
		selfTestResult = null;
		try {
			selfTestResult = await runSelfTest(repoPath);
		} catch (e) {
			selfTestError = String(e);
		} finally {
			selfTestRunning = false;
		}
	}

	let fpsClass = $derived(
		$debug.fps < 50 ? 'text-red-400' : $debug.fps < 55 ? 'text-yellow-400' : 'text-green-400'
	);
	let ipcColorClass = $derived(
		$avgIpcTime > 100 ? 'text-red-400' : $avgIpcTime > 50 ? 'text-yellow-400' : ''
	);
	let stateClass = $derived($operationState !== 'Idle' ? 'text-yellow-400' : '');
	let memoryStr = $derived($debug.memoryUsed > 0 ? formatBytes($debug.memoryUsed) : null);
	let drawColorClass = $derived(
		$debug.graphDrawTimeMs > 16
			? 'text-red-400'
			: $debug.graphDrawTimeMs > 8
				? 'text-yellow-400'
				: ''
	);
</script>

{#if $debug.visible}
	<div
		class="fixed bottom-12 left-4 z-50 rounded-lg border border-gray-700 bg-gray-900/95 p-3 text-xs font-mono text-gray-300 shadow-xl backdrop-blur-sm"
		role="dialog"
		aria-label="Debug overlay"
	>
		<div class="mb-2 flex items-center justify-between">
			<span class="text-yellow-400 font-bold">{$t('debug.header')}</span>
			<span class="text-gray-500">{$t('debug.close_hint')}</span>
		</div>

		<div class="grid grid-cols-2 gap-x-4 gap-y-1">
			<span class="text-gray-500">{$t('debug.fps')}</span>
			<span class={fpsClass}>
				{$debug.fps}
			</span>

			<span class="text-gray-500">{$t('debug.memory')}</span>
			<span>{memoryStr ?? 'N/A'}</span>

			<span class="text-gray-500">{$t('debug.commits')}</span>
			<span>{$debug.totalCommits} ({$debug.visibleCommits} visible)</span>

			<span class="text-gray-500">{$t('debug.graph')}</span>
			<span>{$debug.graphNodes} nodes, {$debug.graphEdges} edges, {$debug.graphColumns} cols</span>

			<span class="text-gray-500">{$t('debug.stashes')}</span>
			<span>{$debug.graphStashMarkers}</span>

			<span class="text-gray-500">{$t('debug.draw')}</span>
			<span class={drawColorClass}>
				{formatMs($debug.graphDrawTimeMs)}ms
			</span>

			<span class="text-gray-500">{$t('debug.state')}</span>
			<span class={stateClass}>{$operationState}</span>

			<span class="text-gray-500">{$t('debug.avg_ipc')}</span>
			<span class={ipcColorClass}>
				{formatMs($avgIpcTime)}ms
			</span>
		</div>

		{#if $debug.loadPhaseTimings.length > 0}
			<div class="mt-2 border-t border-gray-800 pt-2">
				<div class="text-gray-500 mb-1">{$t('debug.load_phases')}</div>
				<div class="max-h-32 overflow-y-auto">
					{#each $debug.loadPhaseTimings as p, i (i)}
						<div class="flex justify-between">
							<span class="truncate max-w-[180px]">{p.phase}</span>
							<span>
								{#if p.count !== undefined}
									×{p.count}
								{:else}
									{formatMs(p.durationMs)}ms
								{/if}
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if $recentIpcTimings.length > 0}
			<div class="mt-2 border-t border-gray-800 pt-2">
				<div class="text-gray-500 mb-1">{$t('debug.recent_ipc')}</div>
				<div class="max-h-48 overflow-y-auto">
					{#each $recentIpcTimings as t, i (i)}
						<div class="flex justify-between">
							<span class="truncate max-w-[180px]">{t.command}</span>
							<span
								class={t.durationMs > 100
									? 'text-red-400'
									: t.durationMs > 50
										? 'text-yellow-400'
										: ''}
							>
								{formatMs(t.durationMs)}ms
							</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<div class="mt-2 border-t border-gray-800 pt-2">
			<button
				class="w-full rounded px-2 py-1 text-xs font-medium disabled:opacity-50 {selfTestError
					? 'bg-red-900 text-red-300'
					: selfTestResult?.property_checks?.some((c) => c.violation_count > 0)
						? 'bg-yellow-900 text-yellow-300'
						: 'bg-blue-900 text-blue-300'} hover:bg-opacity-80"
				onclick={runTest}
				disabled={!repoPath || selfTestRunning}
			>
				{selfTestRunning ? $t('debug.running') : $t('debug.run_self_test')}
			</button>
			{#if selfTestError}
				<div class="mt-1 text-red-400 text-xs">{selfTestError}</div>
			{/if}
			{#if selfTestResult}
				<div class="mt-1 grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
					<span class="text-gray-500">{$t('debug.timing')}</span>
					<span>{formatMs(selfTestResult.timing_ms)}ms</span>
					<span class="text-gray-500">{$t('debug.nodes')}</span>
					<span>{selfTestResult.node_count}</span>
					<span class="text-gray-500">{$t('debug.edges')}</span>
					<span>{selfTestResult.edge_count}</span>
					<span class="text-gray-500">{$t('debug.columns')}</span>
					<span>{selfTestResult.total_columns}</span>
				</div>
				<div class="mt-1 grid grid-cols-2 gap-x-4 gap-y-1 text-xs border-t border-gray-800 pt-1">
					<span class="text-gray-500">{$t('debug.max_threads')}</span>
					<span>{selfTestResult.max_concurrent_threads}</span>
					<span class="text-gray-500">{$t('debug.col_waste')}</span>
					<span>{selfTestResult.column_waste}</span>
					<span class="text-gray-500">{$t('debug.waypoints')}</span>
					<span
						>{selfTestResult.total_waypoints}
						{$t('debug.max_per_edge', { n: selfTestResult.max_waypoints_per_edge })}</span
					>
					<span class="text-gray-500">{$t('debug.arrow_gaps')}</span>
					<span>{selfTestResult.arrow_gap_count}</span>
					<span class="text-gray-500">{$t('debug.edge_types')}</span>
					<span
						>{selfTestResult.straight_edges}/{selfTestResult.branch_edges}/{selfTestResult.merge_edges}</span
					>
					<span class="text-gray-500">{$t('debug.col_shifts')}</span>
					<span class="truncate max-w-[140px]" title={selfTestResult.column_shift_histogram}>
						{selfTestResult.column_shift_histogram || 'none'}
					</span>
				</div>
				<div class="mt-1 grid grid-cols-2 gap-x-4 gap-y-1 text-xs border-t border-gray-800 pt-1">
					<span class="text-gray-500">{$t('debug.total_commits')}</span>
					<span>{selfTestResult.total_commits}</span>
					<span class="text-gray-500">{$t('debug.merges')}</span>
					<span>{selfTestResult.merge_count}</span>
					<span class="text-gray-500">{$t('debug.longest_chain')}</span>
					<span>{selfTestResult.longest_chain}</span>
					<span class="text-gray-500">{$t('debug.fork_points')}</span>
					<span>{selfTestResult.fork_point_count}</span>
					<span class="text-gray-500">{$t('debug.branching_factor')}</span>
					<span
						class="truncate max-w-[140px]"
						title={JSON.stringify(selfTestResult.branching_factor_histogram)}
					>
						{selfTestResult.branching_factor_histogram.join(',')}
					</span>
				</div>
				{#if selfTestResult.property_checks?.length > 0}
					<div class="mt-1 border-t border-gray-800 pt-1">
						<div class="text-xs text-gray-500 mb-0.5">{$t('debug.property_checks')}</div>
						{#each selfTestResult.property_checks as check, j (j)}
							<div class="grid grid-cols-2 gap-x-4 text-xs">
								<span class="text-gray-500">{check.name}</span>
								<span class={check.violation_count > 0 ? 'text-yellow-400' : 'text-green-400'}>
									{check.violation_count}
								</span>
							</div>
							{#if check.violation_count > 0 && check.sample.length > 0}
								<div class="ml-4 max-h-16 overflow-y-auto text-xs text-yellow-400/70">
									{#each check.sample.slice(0, 5) as msg, j (j)}
										<div class="truncate">{msg}</div>
									{/each}
									{#if check.violation_count > check.sample.length}
										<div class="italic">
											{$t('debug.more_count', {
												count: check.violation_count - check.sample.length
											})}
										</div>
									{/if}
								</div>
							{/if}
						{/each}
					</div>
				{/if}
				{#if selfTestResult.hide_merges_property_checks?.length > 0}
					<div class="mt-1 border-t border-gray-800 pt-1">
						<div class="text-xs text-gray-500 mb-0.5">
							{$t('debug.hide_merges_property_checks')}
						</div>
						{#each selfTestResult.hide_merges_property_checks as check, j (j)}
							<div class="grid grid-cols-2 gap-x-4 text-xs">
								<span class="text-gray-500">{check.name}</span>
								<span class={check.violation_count > 0 ? 'text-yellow-400' : 'text-green-400'}>
									{check.violation_count}
								</span>
							</div>
						{/each}
					</div>
				{/if}
			{/if}
		</div>
	</div>
{/if}
