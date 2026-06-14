<script lang="ts">
	import type { Snippet } from 'svelte';

	let {
		text,
		children,
		position = 'top',
		delay = 500
	}: {
		text: string;
		children: Snippet;
		position?: 'top' | 'bottom' | 'left' | 'right';
		delay?: number;
	} = $props();

	let visible = $state(false);
	let timer: ReturnType<typeof setTimeout> | null = null;

	function show() {
		if (timer) clearTimeout(timer);
		timer = setTimeout(() => (visible = true), delay);
	}

	function hide() {
		if (timer) clearTimeout(timer);
		visible = false;
	}

	const posClasses = $derived(
		position === 'top'
			? 'bottom-full left-1/2 -translate-x-1/2 mb-1'
			: position === 'bottom'
				? 'top-full left-1/2 -translate-x-1/2 mt-1'
				: position === 'left'
					? 'right-full top-1/2 -translate-y-1/2 mr-1'
					: 'left-full top-1/2 -translate-y-1/2 ml-1'
	);
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
	class="relative inline-flex"
	onmouseenter={show}
	onmouseleave={hide}
	onfocusin={show}
	onfocusout={hide}
>
	{@render children()}
	{#if visible}
		<span
			class="pointer-events-none absolute z-50 whitespace-nowrap rounded bg-gray-900 px-2 py-1 text-xs text-gray-200 shadow-lg ring-1 ring-gray-700 {posClasses}"
			role="tooltip"
		>
			{text}
		</span>
	{/if}
</span>
