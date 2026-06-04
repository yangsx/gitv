<script lang="ts">
	let {
		panelHeight = $bindable(300),
		minHeight = 200,
		onDragStart,
		onDragEnd
	}: {
		panelHeight?: number;
		minHeight?: number;
		onDragStart?: () => void;
		onDragEnd?: () => void;
	} = $props();

	let dragging = $state(false);

	function onMouseDown(e: MouseEvent) {
		e.preventDefault();
		dragging = true;
		onDragStart?.();

		const startY = e.clientY;
		const startHeight = panelHeight;

		function onMouseMove(e: MouseEvent) {
			const delta = startY - e.clientY;
			panelHeight = Math.max(minHeight, startHeight + delta);
		}

		function onMouseUp() {
			dragging = false;
			onDragEnd?.();
			window.removeEventListener('mousemove', onMouseMove);
			window.removeEventListener('mouseup', onMouseUp);
		}

		window.addEventListener('mousemove', onMouseMove);
		window.addEventListener('mouseup', onMouseUp);
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="flex h-1 cursor-row-resize items-center justify-center border-y border-gray-700 bg-gray-800 hover:bg-gray-700"
	role="separator"
	aria-orientation="horizontal"
	aria-label="Resize detail panel"
	tabindex="-1"
	onmousedown={onMouseDown}
>
	<div class="h-0.5 w-8 rounded bg-gray-500"></div>
</div>
