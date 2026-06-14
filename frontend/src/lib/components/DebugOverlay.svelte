<script lang="ts">
	import { t } from '$lib/stores/locale';
	import {
		debug,
		avgIpcTime,
		recentIpcTimings,
		debugOverlayEnabled,
		formatBytes
	} from '$lib/stores/debug';
	import { operationState } from '$lib/stores/repository';

	let formatMs = (ms: number) => ms.toFixed(1);

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

{#if $debug.visible && $debugOverlayEnabled}
	<div
		class="fixed top-4 right-4 z-50 rounded-lg border border-gray-700 bg-gray-900/95 p-3 text-xs font-mono text-gray-300 shadow-xl backdrop-blur-sm"
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
							<span>{formatMs(p.durationMs)}ms</span>
						</div>
					{/each}
				</div>
			</div>
		{/if}

		{#if $recentIpcTimings.length > 0}
			<div class="mt-2 border-t border-gray-800 pt-2">
				<div class="text-gray-500 mb-1">{$t('debug.recent_ipc')}</div>
				<div class="max-h-32 overflow-y-auto">
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
	</div>
{/if}
