<script lang="ts">
	import type { FileDiff, WorkingChangesDiff } from '$lib/bindings/types';
	import { getWorkingChangesDiffs, getWorkingChangesCombinedDiff } from '$lib/bindings/commands';
	import DiffViewer from './DiffViewer.svelte';
	import { diffMode, diffWhitespace } from '$lib/stores/preferences';
	import { untrack } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { t } from '$lib/stores/locale';

	interface Props {
		repoPath: string;
		workingChangesDiff: WorkingChangesDiff;
		onclose?: () => void;
	}

	let { repoPath, workingChangesDiff, onclose }: Props = $props();

	type WcMode = 'combined' | 'staged' | 'unstaged';
	let wcMode = $state<WcMode>('combined');
	let localDiffMode = $state($diffMode);
	let localDiffWhitespace = $state($diffWhitespace);
	let viewMode = $state<'unified' | 'side-by-side'>('unified');

	let fileDiffs = new SvelteMap<string, FileDiff>();
	let diffsLoading = $state(false);

	let scrollContainer: HTMLDivElement | undefined = $state();

	const files = $derived(
		wcMode === 'combined'
			? [
					...new Map(
						[...workingChangesDiff.staged, ...workingChangesDiff.unstaged].map((f) => [f.path, f])
					).values()
				]
			: wcMode === 'staged'
				? workingChangesDiff.staged
				: workingChangesDiff.unstaged
	);

	const fileCount = $derived(files.length);

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
		fileDiffs.clear();
		untrack(() => {
			loadDiffs();
		});
	});

	async function loadDiffs() {
		if (files.length === 0) return;
		diffsLoading = true;
		try {
			let diffs: FileDiff[];
			if (wcMode === 'combined') {
				diffs = await getWorkingChangesCombinedDiff(repoPath, localDiffMode, localDiffWhitespace);
			} else {
				diffs = await getWorkingChangesDiffs(
					repoPath,
					wcMode === 'staged',
					localDiffMode,
					localDiffWhitespace
				);
			}
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
	}

	function fileHeaderId(index: number): string {
		return `wc-diff-${index}`;
	}

	function setMode(mode: WcMode) {
		if (mode !== wcMode) {
			wcMode = mode;
		}
	}
</script>

<div class="flex h-full flex-col overflow-hidden">
	<div class="flex items-start justify-between border-b border-gray-800 px-4 py-2">
		<div>
			<div class="text-sm font-semibold text-blue-300">
				{$t('working_changes.title')}
			</div>
			<div class="mt-1 text-xs text-gray-400">
				{$t(fileCount === 1 ? 'working_changes.file_count' : 'working_changes.file_count_plural', {
					count: fileCount
				})}
				<span class="text-gray-600 ml-2">
					({$t('working_changes.staged_count', { count: workingChangesDiff.staged.length })}
					&middot;
					{$t('working_changes.unstaged_count', { count: workingChangesDiff.unstaged.length })})
				</span>
			</div>
		</div>
		{#if onclose}
			<button
				class="rounded px-2 py-1 text-xs text-gray-400 hover:bg-gray-800 hover:text-gray-200"
				onclick={onclose}
				aria-label={$t('common.close')}
			>
				{$t('common.close')}
			</button>
		{/if}
	</div>

	<div class="flex items-center gap-2 border-b border-gray-800 px-4 py-1.5">
		<div
			role="radiogroup"
			aria-label={$t('working_changes.mode_aria')}
			class="flex items-center gap-1"
		>
			<button
				class="rounded px-2 py-0.5 text-xs {wcMode === 'combined'
					? 'bg-blue-700/50 text-blue-300'
					: 'text-gray-400 hover:text-gray-200'}"
				onclick={() => setMode('combined')}
				role="radio"
				aria-checked={wcMode === 'combined'}
			>
				{$t('working_changes.combined')}
			</button>
			<button
				class="rounded px-2 py-0.5 text-xs {wcMode === 'staged'
					? 'bg-green-700/50 text-green-300'
					: 'text-gray-400 hover:text-gray-200'}"
				onclick={() => setMode('staged')}
				role="radio"
				aria-checked={wcMode === 'staged'}
			>
				{$t('working_changes.staged')}
			</button>
			<button
				class="rounded px-2 py-0.5 text-xs {wcMode === 'unstaged'
					? 'bg-orange-700/50 text-orange-300'
					: 'text-gray-400 hover:text-gray-200'}"
				onclick={() => setMode('unstaged')}
				role="radio"
				aria-checked={wcMode === 'unstaged'}
			>
				{$t('working_changes.unstaged')}
			</button>
		</div>
		<div class="ml-auto flex items-center gap-2">
			<div
				role="radiogroup"
				aria-label={$t('commit_detail.toggle_view_mode', { mode: viewMode })}
				class="flex items-center gap-1"
			>
				<button
					class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {viewMode ===
					'unified'
						? 'bg-blue-700/50 text-blue-300'
						: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
					onclick={() => (viewMode = 'unified')}
					role="radio"
					aria-checked={viewMode === 'unified'}
				>
					{$t('commit_detail.unified')}
				</button>
				<button
					class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {viewMode ===
					'side-by-side'
						? 'bg-blue-700/50 text-blue-300'
						: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
					onclick={() => (viewMode = 'side-by-side')}
					role="radio"
					aria-checked={viewMode === 'side-by-side'}
				>
					{$t('commit_detail.side_by_side')}
				</button>
			</div>
			<div class="flex items-center gap-1">
				<span class="text-[10px] text-gray-500">{$t('commit_detail.mode_label')}</span>
				<div
					role="radiogroup"
					aria-label={$t('commit_detail.mode_label')}
					class="flex items-center gap-0.5"
				>
					<button
						class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffMode ===
						'normal'
							? 'bg-blue-700/50 text-blue-300'
							: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
						onclick={() => {
							localDiffMode = 'normal';
							loadDiffs();
						}}
						role="radio"
						aria-checked={localDiffMode === 'normal'}>{$t('preferences.mode_normal')}</button
					>
					<button
						class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffMode ===
						'word-diff'
							? 'bg-blue-700/50 text-blue-300'
							: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
						onclick={() => {
							localDiffMode = 'word-diff';
							loadDiffs();
						}}
						role="radio"
						aria-checked={localDiffMode === 'word-diff'}>{$t('preferences.mode_word_diff')}</button
					>
					<button
						class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffMode ===
						'stat-only'
							? 'bg-blue-700/50 text-blue-300'
							: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
						onclick={() => {
							localDiffMode = 'stat-only';
							loadDiffs();
						}}
						role="radio"
						aria-checked={localDiffMode === 'stat-only'}>{$t('preferences.mode_stat_only')}</button
					>
				</div>
			</div>
			{#if localDiffMode !== 'stat-only'}
				<div class="flex items-center gap-1">
					<span class="text-[10px] text-gray-500">{$t('commit_detail.ws_label')}</span>
					<div
						role="radiogroup"
						aria-label={$t('commit_detail.ws_label')}
						class="flex items-center gap-0.5"
					>
						<button
							class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffWhitespace ===
							'none'
								? 'bg-blue-700/50 text-blue-300'
								: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
							onclick={() => {
								localDiffWhitespace = 'none';
								loadDiffs();
							}}
							role="radio"
							aria-checked={localDiffWhitespace === 'none'}>{$t('preferences.ws_show')}</button
						>
						<button
							class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffWhitespace ===
							'ignore-space-change'
								? 'bg-blue-700/50 text-blue-300'
								: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
							onclick={() => {
								localDiffWhitespace = 'ignore-space-change';
								loadDiffs();
							}}
							role="radio"
							aria-checked={localDiffWhitespace === 'ignore-space-change'}
							>{$t('preferences.ws_space')}</button
						>
						<button
							class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffWhitespace ===
							'ignore-all-space'
								? 'bg-blue-700/50 text-blue-300'
								: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
							onclick={() => {
								localDiffWhitespace = 'ignore-all-space';
								loadDiffs();
							}}
							role="radio"
							aria-checked={localDiffWhitespace === 'ignore-all-space'}
							>{$t('preferences.ws_all')}</button
						>
						<button
							class="whitespace-nowrap rounded px-1.5 py-0.5 text-[11px] transition-colors {localDiffWhitespace ===
							'ignore-blank-lines'
								? 'bg-blue-700/50 text-blue-300'
								: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
							onclick={() => {
								localDiffWhitespace = 'ignore-blank-lines';
								loadDiffs();
							}}
							role="radio"
							aria-checked={localDiffWhitespace === 'ignore-blank-lines'}
							>{$t('preferences.ws_blanks')}</button
						>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<div class="flex-1 overflow-y-auto" bind:this={scrollContainer}>
		{#if diffsLoading}
			<div class="flex items-center justify-center py-4 text-sm text-gray-500">
				{$t('commit_detail.loading')}
			</div>
		{:else if files.length === 0}
			<div class="flex items-center justify-center py-4 text-sm text-gray-500">
				{$t('working_changes.no_changes')}
			</div>
		{:else}
			{#each files as file, i (file.path)}
				{@const diff = fileDiffs.get(file.path)}
				<div id={fileHeaderId(i)} class="border-b border-gray-800">
					<div class="flex items-center gap-2 bg-gray-800/60 px-4 py-1.5 sticky top-0 z-10">
						<span class="font-bold {CHANGE_COLORS[file.change_type] ?? ''}">
							{CHANGE_LETTERS[file.change_type] ?? '?'}
						</span>
						<span class="font-mono text-xs text-gray-300">{file.path}</span>
						{#if !file.is_binary && !file.is_submodule}
							<span class="ml-1 font-mono text-[10px]">
								<span class="text-green-500">{file.additions > 0 ? '+' + file.additions : ''}</span>
								<span class="text-red-500">{file.deletions > 0 ? '-' + file.deletions : ''}</span>
							</span>
						{:else if file.is_binary}
							<span class="text-[10px] text-gray-500">{$t('comparison.binary_label')}</span>
						{:else if file.is_submodule}
							<span class="text-[10px] text-orange-400">{$t('commit_detail.submodule')}</span>
						{/if}
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
							<div class="px-4 py-3 text-xs text-gray-500">
								{$t('commit_detail.binary_not_displayed')}
							</div>
						{:else if diff.hunks.length > 0}
							<div class="p-2">
								<DiffViewer hunks={diff.hunks} {viewMode} />
							</div>
						{:else}
							<div class="px-4 py-3 text-xs text-gray-500">
								{$t('commit_detail.no_content_changes')}
							</div>
						{/if}
						{#if diff.truncated_at != null}
							<div class="flex items-center justify-center gap-3 border-t border-gray-700 py-1.5">
								<span class="text-xs text-gray-500">
									{$t('commit_detail.truncated', { count: diff.truncated_at })}
								</span>
							</div>
						{/if}
					{:else if file.is_binary || file.is_submodule}
						<div class="px-4 py-3 text-xs text-gray-500">
							{file.is_submodule
								? $t('commit_detail.submodule')
								: $t('commit_detail.binary_not_displayed')}
						</div>
					{:else}
						<div class="px-4 py-3 text-xs text-gray-500">{$t('common.loading')}</div>
					{/if}
				</div>
			{/each}
		{/if}
	</div>
</div>
