export interface Command {
	id: string;
	label: string;
	shortcut?: string;
	category?: string;
	action: () => void;
}

const commands: Command[] = [];

export function registerCommand(cmd: Command) {
	const existing = commands.findIndex((c) => c.id === cmd.id);
	if (existing >= 0) {
		commands[existing] = cmd;
	} else {
		commands.push(cmd);
	}
}

export function getCommands(): Command[] {
	return [...commands];
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
	if (!query.trim()) return getCommands();
	const scored = getCommands()
		.map((cmd) => ({
			cmd,
			score:
				fuzzyMatch(query, cmd.label) + (cmd.shortcut ? fuzzyMatch(query, cmd.shortcut) * 0.5 : 0)
		}))
		.filter((s) => s.score > 0)
		.sort((a, b) => b.score - a.score);
	return scored.map((s) => s.cmd);
}
