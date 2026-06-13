type MoveCallback = (_x: number, _y: number) => void;

export function draggable(headerEl: HTMLElement, options?: { onMove?: MoveCallback }) {
	let isDragging = false;
	let offsetX = 0;
	let offsetY = 0;

	function onMouseDown(e: MouseEvent) {
		if (e.target instanceof HTMLElement && e.target.closest('button, a, input, select, textarea')) {
			return;
		}
		isDragging = true;
		offsetX = e.clientX - headerEl.getBoundingClientRect().left;
		offsetY = e.clientY - headerEl.getBoundingClientRect().top;
		headerEl.style.cursor = 'grabbing';
	}

	function onMouseMove(e: MouseEvent) {
		if (!isDragging) return;
		options?.onMove?.(e.clientX - offsetX, e.clientY - offsetY);
	}

	function onMouseUp() {
		if (isDragging) {
			isDragging = false;
			headerEl.style.cursor = '';
		}
	}

	headerEl.addEventListener('mousedown', onMouseDown);
	window.addEventListener('mousemove', onMouseMove);
	window.addEventListener('mouseup', onMouseUp);

	return {
		destroy() {
			headerEl.removeEventListener('mousedown', onMouseDown);
			window.removeEventListener('mousemove', onMouseMove);
			window.removeEventListener('mouseup', onMouseUp);
		}
	};
}
