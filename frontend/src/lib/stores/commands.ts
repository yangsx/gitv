import { writable } from 'svelte/store';

export interface Command {
	id: string;
	label: string;
	shortcut?: string;
	category?: string;
	action: () => void;
}

export const commands = writable<Command[]>([]);

export function registerCommand(cmd: Command) {
	commands.update((cmds) => {
		const existing = cmds.findIndex((c) => c.id === cmd.id);
		if (existing >= 0) {
			return [...cmds.slice(0, existing), cmd, ...cmds.slice(existing + 1)];
		}
		return [...cmds, cmd];
	});
}

export function unregisterCommandsByPrefix(prefix: string) {
	commands.update((cmds) => {
		const result: Command[] = [];
		for (const c of cmds) {
			if (!c.id.startsWith(prefix)) {
				result.push(c);
			}
		}
		return result;
	});
}

export function fuzzyMatch(query: string, text: string): number {
	const q = query.toLowerCase();
	const t = text.toLowerCase();
	let qi = 0;
	let score = 0;
	let lastMatchIdx = -1;

	for (let ti = 0; ti < t.length && qi < q.length; ti++) {
		if (t[ti] === q[qi]) {
			score += ti === lastMatchIdx + 1 ? 2 : 1;
			if (ti === 0 || t[ti - 1] === ' ' || t[ti - 1] === '-') score += 1;
			lastMatchIdx = ti;
			qi++;
		}
	}

	return qi === q.length ? score : 0;
}
