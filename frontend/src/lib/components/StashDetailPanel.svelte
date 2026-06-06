<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { FileDiff, StashSplitDiff, Author } from '$lib/bindings/types';
	import DiffViewer from './DiffViewer.svelte';

	interface Props {
		stashIndex: number;
		stashMessage: string;
		stashAuthor: Author;
		stashTime: string;
		fileCount: number;
		diff: FileDiff | null;
		splitDiff: StashSplitDiff | null;
		splitMode: boolean;
		onSplitToggle: () => void;
		onClose?: () => void;
	}

	let {
		stashIndex,
		stashMessage,
		stashAuthor,
		stashTime,
		fileCount,
		diff,
		splitDiff,
		splitMode,
		onSplitToggle,
		onClose
	}: Props = $props();

	let splitTab = $state<'staged' | 'unstaged'>('staged');

	function formatTime(iso: string): string {
		try {
			return new Date(iso).toLocaleDateString(undefined, {
				month: 'short',
				day: 'numeric',
				year: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return '';
		}
	}
</script>

<div class="flex h-full flex-col overflow-hidden">
	<div class="flex items-start justify-between border-b border-gray-800 px-4 py-2">
		<div>
			<div class="text-sm font-semibold text-amber-300">
				{$t('stash_detail_title', { index: stashIndex, message: stashMessage })}
			</div>
			<div class="mt-1 flex items-center gap-3 text-xs text-gray-400">
				<span>{stashAuthor.name} &lt;{stashAuthor.email}&gt;</span>
				<span>{formatTime(stashTime)}</span>
				<span>
					{$t(fileCount === 1 ? 'sidebar.file_count' : 'sidebar.file_count_plural', {
						count: fileCount
					})}
				</span>
			</div>
		</div>
		{#if onClose}
			<button
				class="rounded px-2 py-1 text-xs text-gray-400 hover:bg-gray-800 hover:text-gray-200"
				onclick={onClose}
				aria-label={$t('common.close')}
			>
				{$t('common.close')}
			</button>
		{/if}
	</div>

	<div class="flex items-center gap-2 border-b border-gray-800 px-4 py-1.5">
		<button
			class="rounded px-2 py-0.5 text-xs {!splitMode
				? 'bg-blue-700/50 text-blue-300'
				: 'text-gray-400 hover:text-gray-200'}"
			onclick={() => {
				if (splitMode) onSplitToggle();
			}}
		>
			{$t('stash_detail_combined')}
		</button>
		<button
			class="rounded px-2 py-0.5 text-xs {splitMode
				? 'bg-blue-700/50 text-blue-300'
				: 'text-gray-400 hover:text-gray-200'}"
			onclick={() => {
				if (!splitMode) onSplitToggle();
			}}
		>
			{$t('stash_detail_split')}
		</button>
	</div>

	<div class="flex-1 overflow-y-auto">
		{#if diff}
			{#if !splitMode}
				<div class="p-2">
					<div
						class="sticky top-0 z-10 flex items-center gap-2 bg-gray-900 px-2 py-1 text-xs text-gray-500 border-b border-gray-800"
					>
						<span class="font-semibold">{diff.old_path || diff.path}</span>
					</div>
					<div class="px-2">
						<DiffViewer hunks={diff.hunks} />
					</div>
				</div>
			{:else if splitDiff}
				<div class="flex border-b border-gray-800">
					<button
						class="flex-1 px-3 py-1.5 text-xs {splitTab === 'staged'
							? 'border-b-2 border-blue-500 text-blue-300'
							: 'text-gray-400 hover:text-gray-200'}"
						onclick={() => (splitTab = 'staged')}
					>
						{$t('stash_detail_staged')}
					</button>
					<button
						class="flex-1 px-3 py-1.5 text-xs {splitTab === 'unstaged'
							? 'border-b-2 border-blue-500 text-blue-300'
							: 'text-gray-400 hover:text-gray-200'}"
						onclick={() => (splitTab = 'unstaged')}
					>
						{$t('stash_detail_unstaged')}
					</button>
				</div>
				<div class="p-2">
					{#if splitTab === 'staged'}
						<div class="px-2">
							<DiffViewer hunks={splitDiff.staged.hunks} />
						</div>
					{:else}
						<div class="px-2">
							<DiffViewer hunks={splitDiff.unstaged.hunks} />
						</div>
					{/if}
				</div>
			{/if}
		{:else}
			<div class="flex items-center justify-center h-full text-sm text-gray-500">
				{$t('common.loading')}
			</div>
		{/if}
	</div>
</div>
