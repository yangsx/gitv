import { writable, get } from 'svelte/store';

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
			cmds[existing] = cmd;
		} else {
			cmds.push(cmd);
		}
		return cmds;
	});
}

export function getCommands(): Command[] {
	return [...get(commands)];
}

export function unregisterCommandsByPrefix(prefix: string) {
	commands.update((cmds) => {
		for (let i = cmds.length - 1; i >= 0; i--) {
			if (cmds[i].id.startsWith(prefix)) {
				cmds.splice(i, 1);
			}
		}
		return cmds;
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

export function searchCommands(query: string): Command[] {
	const all = get(commands);
	if (!query.trim()) return [...all];
	const scored = all
		.map((cmd) => ({
			cmd,
			score:
				fuzzyMatch(query, cmd.label) + (cmd.shortcut ? fuzzyMatch(query, cmd.shortcut) * 0.5 : 0)
		}))
		.filter((s) => s.score > 0)
		.sort((a, b) => b.score - a.score);
	return scored.map((s) => s.cmd);
}
