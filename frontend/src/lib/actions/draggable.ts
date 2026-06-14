type MoveCallback = (_x: number, _y: number) => void;

export function draggable(headerEl: HTMLElement, options?: { onMove?: MoveCallback }) {
	let isDragging = false;
	let offsetX = 0;
	let offsetY = 0;

	headerEl.tabIndex = 0;

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

	function onKeyDown(e: KeyboardEvent) {
		const step = e.shiftKey ? 50 : 10;
		const rect = headerEl.getBoundingClientRect();
		let dx = 0;
		let dy = 0;
		switch (e.key) {
			case 'ArrowLeft':
				dx = -step;
				break;
			case 'ArrowRight':
				dx = step;
				break;
			case 'ArrowUp':
				dy = -step;
				break;
			case 'ArrowDown':
				dy = step;
				break;
			default:
				return;
		}
		e.preventDefault();
		options?.onMove?.(rect.left + dx, rect.top + dy);
	}

	headerEl.addEventListener('mousedown', onMouseDown);
	headerEl.addEventListener('keydown', onKeyDown);
	window.addEventListener('mousemove', onMouseMove);
	window.addEventListener('mouseup', onMouseUp);

	return {
		destroy() {
			headerEl.removeEventListener('mousedown', onMouseDown);
			headerEl.removeEventListener('keydown', onKeyDown);
			window.removeEventListener('mousemove', onMouseMove);
			window.removeEventListener('mouseup', onMouseUp);
		}
	};
}
