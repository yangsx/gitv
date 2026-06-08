<script lang="ts">
	let {
		panelHeight = $bindable(300),
		rightPanelWidth = $bindable(240),
		minHeight = 200,
		maxHeight = Infinity,
		minWidth = 160,
		maxWidth = 400,
		direction = 'vertical',
		onDragStart,
		onDragEnd
	}: {
		panelHeight?: number;
		rightPanelWidth?: number;
		minHeight?: number;
		maxHeight?: number;
		minWidth?: number;
		maxWidth?: number;
		direction?: 'vertical' | 'horizontal';
		onDragStart?: () => void;
		onDragEnd?: () => void;
	} = $props();

	function onKeyDown(e: KeyboardEvent) {
		if (isVertical()) {
			if (e.key === 'ArrowUp') {
				e.preventDefault();
				panelHeight = Math.max(minHeight, Math.min(maxHeight, panelHeight + 20));
			} else if (e.key === 'ArrowDown') {
				e.preventDefault();
				panelHeight = Math.max(minHeight, Math.min(maxHeight, panelHeight - 20));
			}
		} else {
			if (e.key === 'ArrowLeft') {
				e.preventDefault();
				rightPanelWidth = Math.max(minWidth, Math.min(maxWidth, rightPanelWidth + 20));
			} else if (e.key === 'ArrowRight') {
				e.preventDefault();
				rightPanelWidth = Math.max(minWidth, Math.min(maxWidth, rightPanelWidth - 20));
			}
		}
	}

	function isVertical(): boolean {
		return direction === 'vertical';
	}

	function onMouseDown(e: MouseEvent) {
		e.preventDefault();
		onDragStart?.();

		const startX = e.clientX;
		const startY = e.clientY;
		const startWidth = rightPanelWidth;
		const startHeight = panelHeight;

		function onMouseMove(e: MouseEvent) {
			if (isVertical()) {
				const delta = startY - e.clientY;
				panelHeight = Math.max(minHeight, Math.min(maxHeight, startHeight + delta));
			} else {
				const delta = startX - e.clientX;
				rightPanelWidth = Math.max(minWidth, Math.min(maxWidth, startWidth + delta));
			}
		}

		function onMouseUp() {
			onDragEnd?.();
			window.removeEventListener('mousemove', onMouseMove);
			window.removeEventListener('mouseup', onMouseUp);
		}

		window.addEventListener('mousemove', onMouseMove);
		window.addEventListener('mouseup', onMouseUp);
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	class="flex {isVertical()
		? 'h-1 cursor-row-resize border-y'
		: 'w-1 cursor-col-resize border-x'} items-center justify-center border-gray-700 bg-gray-800 hover:bg-gray-700 {isVertical()
		? ''
		: 'flex-col'}"
	role="separator"
	aria-orientation={isVertical() ? 'horizontal' : 'vertical'}
	aria-label={isVertical() ? 'Resize detail panel' : 'Resize file list'}
	tabindex={0}
	onmousedown={onMouseDown}
	onkeydown={onKeyDown}
>
	<div class="rounded bg-gray-500 {isVertical() ? 'h-0.5 w-8' : 'w-0.5 h-8'}"></div>
</div>
