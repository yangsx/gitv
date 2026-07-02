import { describe, it, expect } from 'vitest';
import { parseSemver, compareSemverDesc, sortTagNamesDesc } from './semver';

describe('parseSemver', () => {
	it('parses simple version', () => {
		expect(parseSemver('v1.2.3')).toEqual({ parts: [1, 2, 3, 0], pre: null });
	});

	it('parses without v prefix', () => {
		expect(parseSemver('1.2.3')).toEqual({ parts: [1, 2, 3, 0], pre: null });
	});

	it('parses two-part version', () => {
		expect(parseSemver('v1.88')).toEqual({ parts: [1, 88, 0, 0], pre: null });
	});

	it('parses four-part version', () => {
		expect(parseSemver('v1.2.3.4')).toEqual({ parts: [1, 2, 3, 4], pre: null });
	});

	it('parses pre-release with dash', () => {
		expect(parseSemver('v1.88.0-dev.1')).toEqual({
			parts: [1, 88, 0, 0],
			pre: 'dev.1'
		});
	});

	it('parses pre-release with dot', () => {
		expect(parseSemver('v1.88.0.rc.1')).toEqual({
			parts: [1, 88, 0, 0],
			pre: 'rc.1'
		});
	});

	it('parses pre-release with no separator', () => {
		expect(parseSemver('v1.88.0rc2')).toEqual({
			parts: [1, 88, 0, 0],
			pre: 'rc2'
		});
	});

	it('returns null for non-version tag', () => {
		expect(parseSemver('latest')).toBeNull();
	});

	it('returns null for release-like name', () => {
		expect(parseSemver('release-2024')).toBeNull();
	});
});

describe('compareSemverDesc', () => {
	it('higher version sorts first', () => {
		const a = parseSemver('v1.0.0')!;
		const b = parseSemver('v2.0.0')!;
		expect(compareSemverDesc(a, b)).toBeGreaterThan(0);
	});

	it('equal versions return 0', () => {
		const a = parseSemver('v1.0.0')!;
		const b = parseSemver('v1.0.0')!;
		expect(compareSemverDesc(a, b)).toBe(0);
	});

	it('release sorts before pre-release', () => {
		const rel = parseSemver('v1.88.0')!;
		const pre = parseSemver('v1.88.0-dev.1')!;
		expect(compareSemverDesc(rel, pre)).toBeLessThan(0);
	});
});

describe('sortTagNamesDesc', () => {
	it('sorts version tags descending', () => {
		const result = sortTagNamesDesc(['v1.0.0', 'v3.0.0', 'v2.0.0']);
		expect(result).toEqual(['v3.0.0', 'v2.0.0', 'v1.0.0']);
	});

	it('sorts pre-releases after release', () => {
		const result = sortTagNamesDesc(['v1.88.0-dev.1', 'v1.88.0', 'v1.88.0.rc.1', 'v1.88.0rc2']);
		expect(result).toEqual(['v1.88.0', 'v1.88.0rc2', 'v1.88.0.rc.1', 'v1.88.0-dev.1']);
	});

	it('sorts mixed version and non-version tags', () => {
		const result = sortTagNamesDesc(['latest', 'v2.0.0', 'v1.0.0', 'nightly']);
		expect(result.slice(0, 2)).toEqual(['v2.0.0', 'v1.0.0']);
	});

	it('sorts non-version tags alphabetically after version tags', () => {
		const result = sortTagNamesDesc(['nightly', 'latest', 'v1.0.0']);
		expect(result).toEqual(['v1.0.0', 'latest', 'nightly']);
	});

	it('handles cross-version pre-releases', () => {
		const result = sortTagNamesDesc(['v2.0.0-rc.1', 'v1.0.0', 'v2.0.0', 'v1.0.0-beta.1']);
		expect(result).toEqual(['v2.0.0', 'v2.0.0-rc.1', 'v1.0.0', 'v1.0.0-beta.1']);
	});

	it('handles four-part versions', () => {
		const result = sortTagNamesDesc(['v1.2.3.4', 'v1.2.3.5', 'v1.2.3.3']);
		expect(result).toEqual(['v1.2.3.5', 'v1.2.3.4', 'v1.2.3.3']);
	});
});
