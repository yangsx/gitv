<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { GraphLayout } from '$lib/bindings/types';
	import { graphColorMode } from '$lib/stores/repository';

	let { layout }: { layout: GraphLayout } = $props();

	let authorColors = $derived.by(() => {
		if (!$graphColorMode || $graphColorMode === 'by-branch') return [];
		const colors: string[] = [];
		for (const node of layout.nodes) {
			const css = `rgb(${node.color.r},${node.color.g},${node.color.b})`;
			if (!colors.includes(css)) {
				colors.push(css);
			}
			if (colors.length >= 20) break;
		}
		return colors;
	});
</script>

{#if $graphColorMode === 'by-author' && authorColors.length > 0}
	<div class="flex items-center gap-1 text-[10px] text-gray-400">
		<span>{$t('author_legend.label')}</span>
		<div class="flex gap-0.5">
			{#each authorColors as color (color)}
				<span
					class="inline-block h-2.5 w-2.5 rounded-full"
					style="background-color: {color}"
					title={$t('author_legend.title')}
					aria-label={$t('author_legend.title')}
				></span>
			{/each}
		</div>
	</div>
{/if}
