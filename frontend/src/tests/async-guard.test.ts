import { describe, it, expect } from 'vitest';
import { createGenerationGuard } from '$lib/utils/async-guard';

describe('createGenerationGuard', () => {
	it('starts at generation 0', () => {
		const guard = createGenerationGuard();
		expect(guard.current()).toBe(0);
	});

	it('next() increments and returns the new generation', () => {
		const guard = createGenerationGuard();
		expect(guard.next()).toBe(1);
		expect(guard.current()).toBe(1);
		expect(guard.next()).toBe(2);
	});

	it('isStale returns false for the active generation', () => {
		const guard = createGenerationGuard();
		const gen = guard.next();
		expect(guard.isStale(gen)).toBe(false);
	});

	it('isStale returns true for an old generation after advance', () => {
		const guard = createGenerationGuard();
		const oldGen = guard.next();
		guard.next();
		expect(guard.isStale(oldGen)).toBe(true);
	});

	it('isStale returns true even after multiple advances', () => {
		const guard = createGenerationGuard();
		const oldGen = guard.next();
		guard.next();
		guard.next();
		guard.next();
		expect(guard.isStale(oldGen)).toBe(true);
	});

	it('multiple independent guards do not interfere', () => {
		const a = createGenerationGuard();
		const b = createGenerationGuard();

		const genA = a.next();
		b.next();

		expect(a.isStale(genA)).toBe(false);
		expect(a.current()).toBe(1);
		expect(b.current()).toBe(1);
	});
});
