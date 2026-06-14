const OFFSET_STEP = 28;
const activeIndices = new Set<number>();

export function dialogStackOffset(): { offset: number; unregister: () => void } {
	let index = 0;
	while (activeIndices.has(index)) index++;
	activeIndices.add(index);
	const offset = index * OFFSET_STEP;
	return {
		offset,
		unregister: () => {
			activeIndices.delete(index);
		}
	};
}
