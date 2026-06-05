import { writable, get } from 'svelte/store';
import { getPreferences, setPreferences } from '$lib/bindings/commands';
import type { AppPreferences } from '$lib/bindings/types';
import { graphColorMode, graphHideMerges, graphOrientation, graphPalette } from './repository';

const DEFAULTS: AppPreferences = {
	graph_color_mode: 'by-branch',
	graph_hide_merges: false,
	graph_orientation: 'top-to-bottom',
	graph_palette: 'default',
	diff_mode: 'normal',
	diff_whitespace: 'none',
	theme: 'dark'
};

export const theme = writable<'dark' | 'light'>(DEFAULTS.theme as 'dark' | 'light');
export const diffMode = writable<'normal' | 'word-diff' | 'stat-only'>(
	DEFAULTS.diff_mode as 'normal' | 'word-diff' | 'stat-only'
);
export const diffWhitespace = writable<
	'none' | 'ignore-space-change' | 'ignore-all-space' | 'ignore-blank-lines'
>(
	DEFAULTS.diff_whitespace as
		| 'none'
		| 'ignore-space-change'
		| 'ignore-all-space'
		| 'ignore-blank-lines'
);
let loadedPrefs: AppPreferences | null = null;
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function toPreferences(): AppPreferences {
	return {
		graph_color_mode: get(graphColorMode),
		graph_hide_merges: get(graphHideMerges),
		graph_orientation: get(graphOrientation),
		graph_palette: get(graphPalette),
		diff_mode: get(diffMode),
		diff_whitespace: get(diffWhitespace),
		theme: get(theme)
	};
}

function updateFromPreferences(p: AppPreferences) {
	loadedPrefs = p;

	theme.set(p.theme);
	diffMode.set(p.diff_mode);
	diffWhitespace.set(p.diff_whitespace);
	if (p.graph_color_mode === 'by-branch' || p.graph_color_mode === 'by-author') {
		graphColorMode.set(p.graph_color_mode);
	}
	graphHideMerges.set(p.graph_hide_merges);
	if (p.graph_orientation === 'top-to-bottom' || p.graph_orientation === 'bottom-to-top') {
		graphOrientation.set(p.graph_orientation);
	}
	if (
		p.graph_palette === 'default' ||
		p.graph_palette === 'deuteranopia' ||
		p.graph_palette === 'protanopia' ||
		p.graph_palette === 'tritanopia'
	) {
		graphPalette.set(p.graph_palette);
	}
}

function debouncedSave(prefs: AppPreferences) {
	if (saveTimer) clearTimeout(saveTimer);
	saveTimer = setTimeout(async () => {
		try {
			await setPreferences(prefs);
		} catch {
			// save failed — non-critical
		}
	}, 300);
}

export async function initPreferences() {
	try {
		const prefs = await getPreferences();
		updateFromPreferences(prefs);
	} catch {
		// use defaults
	}
}

export function savePreferences() {
	const prefs = toPreferences();
	debouncedSave(prefs);
}

export function getCurrentPreferences(): AppPreferences {
	if (loadedPrefs) return { ...loadedPrefs };
	return { ...DEFAULTS };
}
