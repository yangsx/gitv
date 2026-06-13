export interface GenerationGuard {
	next(): number;
	current(): number;
	isStale(_gen: number): boolean;
}

export function createGenerationGuard(): GenerationGuard {
	let generation = 0;
	return {
		next(): number {
			return ++generation;
		},
		current(): number {
			return generation;
		},
		isStale(_gen: number): boolean {
			return _gen !== generation;
		}
	};
}
