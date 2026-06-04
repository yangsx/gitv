<script lang="ts">
	import type { DiffLine, Hunk, WordDiffSegment } from '$lib/bindings/types';

	interface Props {
		hunks: Hunk[];
	}

	let { hunks }: Props = $props();

	function lineKind(dl: DiffLine): 'context' | 'addition' | 'deletion' | 'worddiff' {
		if ('Context' in dl) return 'context';
		if ('Addition' in dl) return 'addition';
		if ('Deletion' in dl) return 'deletion';
		return 'worddiff';
	}

	function lineContent(dl: DiffLine): string {
		if ('Context' in dl) return dl.Context.content;
		if ('Addition' in dl) return dl.Addition.content;
		if ('Deletion' in dl) return dl.Deletion.content;
		return dl.WordDiff.content;
	}

	function oldLineNum(dl: DiffLine): string {
		if ('Context' in dl) return String(dl.Context.old_line);
		if ('Deletion' in dl) return String(dl.Deletion.old_line);
		if ('WordDiff' in dl) return String(dl.WordDiff.old_line);
		return '';
	}

	function newLineNum(dl: DiffLine): string {
		if ('Context' in dl) return String(dl.Context.new_line);
		if ('Addition' in dl) return String(dl.Addition.new_line);
		if ('WordDiff' in dl) return String(dl.WordDiff.new_line);
		return '';
	}

	function wordDiffSegments(dl: DiffLine): WordDiffSegment[] | null {
		if ('WordDiff' in dl) return dl.WordDiff.segments;
		return null;
	}

	function segmentClass(kind: string): string {
		if (kind === 'Added') return 'bg-green-500/30';
		if (kind === 'Removed') return 'bg-red-500/30';
		return '';
	}
</script>

<div class="font-mono text-xs leading-5">
	{#each hunks as hunk}
		<div class="border-t border-gray-700 bg-gray-800/50 px-2 py-0.5 text-gray-400">
			@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count}
		</div>
		{#each hunk.lines as line}
			{@const kind = lineKind(line)}
			{@const bgClass =
				kind === 'addition'
					? 'bg-green-900/30'
					: kind === 'deletion'
						? 'bg-red-900/30'
						: kind === 'worddiff'
							? 'bg-yellow-900/20'
							: ''}
			<div class="flex {bgClass}">
				<span class="w-12 shrink-0 select-none text-right text-gray-600">{oldLineNum(line)}</span>
				<span class="w-12 shrink-0 select-none text-right text-gray-600">{newLineNum(line)}</span>
				<span
					class="shrink-0 w-4 text-center select-none"
					class:text-green-400={kind === 'addition'}
					class:text-red-400={kind === 'deletion'}
					class:text-yellow-400={kind === 'worddiff'}
					class:text-gray-500={kind === 'context'}
				>
					{#if kind === 'addition'}+{:else if kind === 'deletion'}-{:else if kind === 'worddiff'}~{:else}&nbsp;{/if}
				</span>
				{#if wordDiffSegments(line)}
					<pre
						class="whitespace-pre-wrap break-all flex-1 min-w-0">{#each wordDiffSegments(line)! as seg, si (si)}<span
								class={segmentClass(seg.kind)}>{seg.text}</span
							>{/each}</pre>
				{:else}
					<pre class="whitespace-pre-wrap break-all flex-1 min-w-0">{lineContent(line)}</pre>
				{/if}
			</div>
		{/each}
	{/each}
</div>
