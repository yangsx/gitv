<script lang="ts">
	interface MenuItem {
		label: string;
		shortcut?: string;
		action: () => void;
		separator?: false;
	}

	interface Separator {
		separator: true;
	}

	export type ContextMenuItem = MenuItem | Separator;

	interface Props {
		x: number;
		y: number;
		items: ContextMenuItem[];
		onclose: () => void;
	}

	let { x, y, items, onclose }: Props = $props();

	let menuEl: HTMLDivElement | undefined = $state();
	let adjustedX = $state(0);
	let adjustedY = $state(0);
	let activeIdx = $state(0);

	const menuItems = $derived(
		items.map((item, i) => ({ item, i })).filter((x) => !('separator' in x.item))
	);

	$effect(() => {
		if (!menuEl) return;
		const rect = menuEl.getBoundingClientRect();
		const vw = window.innerWidth;
		const vh = window.innerHeight;
		if (x + rect.width > vw) adjustedX = vw - rect.width - 8;
		else adjustedX = x;
		if (y + rect.height > vh) adjustedY = vh - rect.height - 8;
		else adjustedY = y;
	});

	$effect(() => {
		activeIdx = 0;
		const first = menuEl?.querySelector<HTMLButtonElement>('[role="menuitem"]');
		first?.focus();
	});

	function focusItem(idx: number) {
		const buttons = menuEl?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]');
		buttons?.[idx]?.focus();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
		} else if (menuItems.length > 0 && e.key === 'ArrowDown') {
			e.preventDefault();
			activeIdx = (activeIdx + 1) % menuItems.length;
			focusItem(activeIdx);
		} else if (menuItems.length > 0 && e.key === 'ArrowUp') {
			e.preventDefault();
			activeIdx = (activeIdx - 1 + menuItems.length) % menuItems.length;
			focusItem(activeIdx);
		}
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="fixed inset-0 z-50"
	onclick={onBackdropClick}
	oncontextmenu={(e) => {
		e.preventDefault();
		onclose();
	}}
	role="presentation"
>
	<div
		bind:this={menuEl}
		class="fixed rounded-md border border-gray-700 bg-gray-900 py-1 shadow-xl"
		style="left: {adjustedX}px; top: {adjustedY}px; min-width: 180px;"
		role="menu"
	>
		{#if menuItems.length === 0}
			<div class="px-3 py-1.5 text-xs text-gray-500 italic">—</div>
		{:else}
			{#each items as item, i (i)}
				{#if 'separator' in item && item.separator}
					<div class="my-1 border-t border-gray-800"></div>
				{:else if !('separator' in item)}
					<button
						class="flex w-full items-center justify-between px-3 py-1.5 text-xs text-gray-300 hover:bg-blue-600/30 hover:text-white"
						onclick={() => {
							item.action();
							onclose();
						}}
						role="menuitem"
					>
						<span>{item.label}</span>
						{#if item.shortcut}
							<span class="ml-4 text-gray-500">{item.shortcut}</span>
						{/if}
					</button>
				{/if}
			{/each}
		{/if}
	</div>
</div>
