<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { DiffLine, Hunk, WordDiffSegment } from '$lib/bindings/types';

	interface Props {
		hunks: Hunk[];
		viewMode?: 'unified' | 'side-by-side';
	}

	let { hunks, viewMode = 'unified' }: Props = $props();

	function lineKind(dl: DiffLine): 'context' | 'addition' | 'deletion' | 'worddiff' {
		if (!dl || typeof dl !== 'object') return 'context';
		if ('Context' in dl) return 'context';
		if ('Addition' in dl) return 'addition';
		if ('Deletion' in dl) return 'deletion';
		return 'worddiff';
	}

	function lineContent(dl: DiffLine): string {
		if (!dl || typeof dl !== 'object') return '';
		if ('Context' in dl) return dl.Context.content;
		if ('Addition' in dl) return dl.Addition.content;
		if ('Deletion' in dl) return dl.Deletion.content;
		return dl.WordDiff.content;
	}

	function oldLineNum(dl: DiffLine): string {
		if (!dl || typeof dl !== 'object') return '';
		if ('Context' in dl) return String(dl.Context.old_line);
		if ('Deletion' in dl) return String(dl.Deletion.old_line);
		if ('WordDiff' in dl) return String(dl.WordDiff.old_line);
		return '';
	}

	function newLineNum(dl: DiffLine): string {
		if (!dl || typeof dl !== 'object') return '';
		if ('Context' in dl) return String(dl.Context.new_line);
		if ('Addition' in dl) return String(dl.Addition.new_line);
		if ('WordDiff' in dl) return String(dl.WordDiff.new_line);
		return '';
	}

	function wordDiffSegments(dl: DiffLine): WordDiffSegment[] | null {
		if (!dl || typeof dl !== 'object') return null;
		if ('WordDiff' in dl) return dl.WordDiff.segments;
		return null;
	}

	function segmentClass(kind: string): string {
		if (kind === 'Added') return 'bg-green-500/30';
		if (kind === 'Removed') return 'bg-red-500/30';
		return '';
	}

	type Side = 'left' | 'right';

	function filterWordDiffSegments(segments: WordDiffSegment[], side: Side): WordDiffSegment[] {
		return segments.filter((s) => {
			if (s.kind === 'Unchanged') return true;
			return side === 'left' ? s.kind === 'Removed' : s.kind === 'Added';
		});
	}

	function splitWordDiff(line: DiffLine, side: Side): DiffLine {
		if (!('WordDiff' in line)) return line;
		const wd = line.WordDiff;
		const filtered = filterWordDiffSegments(wd.segments, side);
		return {
			WordDiff: {
				content: filtered.map((s) => s.text).join(''),
				old_line: side === 'left' ? wd.old_line : 0,
				new_line: side === 'right' ? wd.new_line : 0,
				segments: filtered
			}
		};
	}

	function splitHunkLines(lines: DiffLine[]): {
		left: (DiffLine | null)[];
		right: (DiffLine | null)[];
	} {
		const left: (DiffLine | null)[] = [];
		const right: (DiffLine | null)[] = [];

		let i = 0;
		while (i < lines.length) {
			const line = lines[i];
			const kind = lineKind(line);
			if (kind === 'context') {
				left.push(line);
				right.push(line);
				i++;
			} else if (kind === 'worddiff') {
				left.push(splitWordDiff(line, 'left'));
				right.push(splitWordDiff(line, 'right'));
				i++;
			} else if (kind === 'deletion') {
				const deletions: DiffLine[] = [];
				while (i < lines.length && lineKind(lines[i]) === 'deletion') {
					deletions.push(lines[i]);
					i++;
				}
				const additions: DiffLine[] = [];
				while (i < lines.length && lineKind(lines[i]) === 'addition') {
					additions.push(lines[i]);
					i++;
				}
				const maxLen = Math.max(deletions.length, additions.length);
				for (let j = 0; j < maxLen; j++) {
					left.push(j < deletions.length ? deletions[j] : null);
					right.push(j < additions.length ? additions[j] : null);
				}
			} else if (kind === 'addition') {
				left.push(null);
				right.push(line);
				i++;
			} else {
				i++;
			}
		}
		return { left, right };
	}
</script>

{#if viewMode === 'side-by-side'}
	<div
		class="font-mono text-xs leading-5"
		role="region"
		aria-label={$t('diff_viewer.side_by_side_aria')}
	>
		{#each hunks as hunk, hi (hi)}
			{@const oldEnd = hunk.old_start + hunk.old_count}
			{@const newEnd = hunk.new_start + hunk.new_count}
			<div
				class="border-t border-gray-700 bg-gray-800/50 px-2 py-0.5 text-gray-400"
				aria-label={$t('diff_viewer.hunk_aria', {
					old_start: hunk.old_start,
					old_end: oldEnd,
					new_start: hunk.new_start,
					new_end: newEnd
				})}
			>
				@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count}
			</div>
			{@const split = splitHunkLines(hunk.lines)}
			{#each split.left as leftLine, idx (idx)}
				{@const rightLine = split.right[idx]}
				<div class="flex">
					<div
						class="flex flex-1 min-w-0 {leftLine && lineKind(leftLine) === 'deletion'
							? 'bg-red-900/30'
							: ''} {leftLine && lineKind(leftLine) === 'worddiff'
							? 'bg-red-900/20'
							: ''} {leftLine === null ? 'bg-gray-800/30' : ''}"
					>
						<span class="w-10 shrink-0 select-none text-right text-gray-600"
							>{leftLine ? oldLineNum(leftLine) : ''}</span
						>
						<span
							class="w-4 shrink-0 select-none text-center {leftLine &&
							(lineKind(leftLine) === 'deletion' || lineKind(leftLine) === 'worddiff')
								? 'text-red-400'
								: 'text-gray-500'}"
							>{leftLine && (lineKind(leftLine) === 'deletion' || lineKind(leftLine) === 'worddiff')
								? '-'
								: '\u00a0'}</span
						>
						{#if leftLine && wordDiffSegments(leftLine)}
							<pre
								class="whitespace-pre-wrap break-all flex-1 min-w-0">{#each wordDiffSegments(leftLine)! as seg, si (si)}<span
										class={segmentClass(seg.kind)}>{seg.text}</span
									>{/each}</pre>
						{:else}
							<pre class="whitespace-pre-wrap break-all flex-1 min-w-0">{leftLine
									? lineContent(leftLine)
									: ''}</pre>
						{/if}
					</div>
					<div class="w-px shrink-0 bg-gray-700"></div>
					<div
						class="flex flex-1 min-w-0 {rightLine && lineKind(rightLine) === 'addition'
							? 'bg-green-900/30'
							: ''} {rightLine && lineKind(rightLine) === 'worddiff'
							? 'bg-green-900/20'
							: ''} {rightLine === null ? 'bg-gray-800/30' : ''}"
					>
						<span class="w-10 shrink-0 select-none text-right text-gray-600"
							>{rightLine ? newLineNum(rightLine) : ''}</span
						>
						<span
							class="w-4 shrink-0 select-none text-center {rightLine &&
							(lineKind(rightLine) === 'addition' || lineKind(rightLine) === 'worddiff')
								? 'text-green-400'
								: 'text-gray-500'}"
							>{rightLine &&
							(lineKind(rightLine) === 'addition' || lineKind(rightLine) === 'worddiff')
								? '+'
								: '\u00a0'}</span
						>
						{#if rightLine && wordDiffSegments(rightLine)}
							<pre
								class="whitespace-pre-wrap break-all flex-1 min-w-0">{#each wordDiffSegments(rightLine)! as seg, si (si)}<span
										class={segmentClass(seg.kind)}>{seg.text}</span
									>{/each}</pre>
						{:else}
							<pre class="whitespace-pre-wrap break-all flex-1 min-w-0">{rightLine
									? lineContent(rightLine)
									: ''}</pre>
						{/if}
					</div>
				</div>
			{/each}
		{/each}
	</div>
{:else}
	<div
		class="font-mono text-xs leading-5"
		role="region"
		aria-label={$t('diff_viewer.unified_aria')}
	>
		{#each hunks as hunk, hi (hi)}
			{@const oldEnd = hunk.old_start + hunk.old_count}
			{@const newEnd = hunk.new_start + hunk.new_count}
			<div
				class="border-t border-gray-700 bg-gray-800/50 px-2 py-0.5 text-gray-400"
				aria-label={$t('diff_viewer.hunk_aria', {
					old_start: hunk.old_start,
					old_end: oldEnd,
					new_start: hunk.new_start,
					new_end: newEnd
				})}
			>
				@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count}
			</div>
			{#each hunk.lines as line, li (li)}
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
{/if}
