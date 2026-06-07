import { writable, derived } from 'svelte/store';

export interface IpcTiming {
	command: string;
	durationMs: number;
	timestamp: number;
}

interface DebugState {
	visible: boolean;
	fps: number;
	ipcTimings: IpcTiming[];
	totalCommits: number;
	visibleCommits: number;
	graphNodes: number;
	graphEdges: number;
	graphStashMarkers: number;
	graphColumns: number;
	memoryUsed: number;
}

const initialState: DebugState = {
	visible: false,
	fps: 0,
	ipcTimings: [],
	totalCommits: 0,
	visibleCommits: 0,
	graphNodes: 0,
	graphEdges: 0,
	graphStashMarkers: 0,
	graphColumns: 0,
	memoryUsed: 0
};

export const debug = writable<DebugState>(initialState);

export const avgIpcTime = derived(debug, ($d) => {
	if ($d.ipcTimings.length === 0) return 0;
	const sum = $d.ipcTimings.reduce((a, t) => a + t.durationMs, 0);
	return Math.round((sum / $d.ipcTimings.length) * 100) / 100;
});

export const recentIpcTimings = derived(debug, ($d) => {
	return $d.ipcTimings.slice(-20);
});

export const debugOverlayEnabled = writable(false);
export const logPath = writable('');

const MAX_TIMINGS = 100;

export function recordIpcTiming(command: string, durationMs: number) {
	debug.update((d) => {
		const timings = [...d.ipcTimings, { command, durationMs, timestamp: Date.now() }];
		return { ...d, ipcTimings: timings.slice(-MAX_TIMINGS) };
	});
}

export function updateDebugGraphStats(
	nodes: number,
	edges: number,
	stashMarkers: number,
	columns: number
) {
	debug.update((d) => ({
		...d,
		graphNodes: nodes,
		graphEdges: edges,
		graphStashMarkers: stashMarkers,
		graphColumns: columns
	}));
}

export function updateDebugCommitCounts(total: number, visible: number) {
	debug.update((d) => ({ ...d, totalCommits: total, visibleCommits: visible }));
}

export function toggleDebug() {
	debug.update((d) => ({ ...d, visible: !d.visible }));
}

let fpsFrames = 0;
let fpsLastTime = performance.now();

export function tickFps() {
	fpsFrames++;
	const now = performance.now();
	if (now - fpsLastTime >= 1000) {
		debug.update((d) => ({ ...d, fps: Math.round((fpsFrames * 1000) / (now - fpsLastTime)) }));
		fpsFrames = 0;
		fpsLastTime = now;
	}
}

let memoryTimer: ReturnType<typeof setInterval> | null = null;

export function startMemoryTracking() {
	if (memoryTimer) return;
	memoryTimer = setInterval(() => {
		const mem = (performance as unknown as Record<string, unknown>).memory as
			| { usedJSHeapSize: number }
			| undefined;
		if (mem?.usedJSHeapSize) {
			debug.update((d) => ({ ...d, memoryUsed: mem.usedJSHeapSize }));
		}
	}, 2000);
}

export function stopMemoryTracking() {
	if (memoryTimer) {
		clearInterval(memoryTimer);
		memoryTimer = null;
	}
}

export function formatBytes(bytes: number): string {
	if (bytes === 0) return '0 B';
	const units = ['B', 'KB', 'MB', 'GB'];
	const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
	return (bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1) + ' ' + units[i];
}
