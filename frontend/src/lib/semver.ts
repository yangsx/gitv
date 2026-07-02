export interface ParsedVersion {
	parts: number[];
	pre: string | null;
}

export function parseSemver(name: string): ParsedVersion | null {
	const m = name.match(/^v?(\d+)\.(\d+)(?:\.(\d+))?(?:\.(\d+))?(.*)/i);
	if (!m) return null;
	const parts = [m[1], m[2], m[3] ?? '0', m[4] ?? '0'].map(Number);
	let pre = m[5] ?? '';
	if (pre) {
		pre = pre.replace(/^[-.\s]+/, '').toLowerCase();
	}
	return { parts, pre: pre || null };
}

export function compareSemverDesc(a: ParsedVersion, b: ParsedVersion): number {
	for (let i = 0; i < Math.max(a.parts.length, b.parts.length); i++) {
		const diff = (b.parts[i] ?? 0) - (a.parts[i] ?? 0);
		if (diff !== 0) return diff;
	}
	if (a.pre && b.pre) return b.pre.localeCompare(a.pre);
	if (a.pre) return 1;
	if (b.pre) return -1;
	return 0;
}

export function sortTagNamesDesc(names: string[]): string[] {
	return [...names].sort((a, b) => {
		const va = parseSemver(a);
		const vb = parseSemver(b);
		if (va && vb) return compareSemverDesc(va, vb);
		if (va) return -1;
		if (vb) return 1;
		return a.localeCompare(b);
	});
}
