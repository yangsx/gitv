import { writable, get, derived } from 'svelte/store';
import { getPreferences, setPreferences } from '$lib/bindings/commands';
import type { AppPreferences } from '$lib/bindings/types';
import {
	graphColorMode,
	graphHideMerges,
	graphOrientation,
	graphPalette,
	arrowGapThreshold
} from './repository';
import { locale, initLocale, setLocale, SUPPORTED_LOCALES, DEFAULT_LOCALE } from './locale';

export const FONT_SIZE_MIN = 10;
export const FONT_SIZE_MAX = 24;
export const FONT_SIZE_DEFAULT = 13;

const DEFAULTS: AppPreferences = {
	graph_color_mode: 'by-branch',
	graph_hide_merges: false,
	graph_orientation: 'top-to-bottom',
	graph_palette: 'default',
	renderer: 'wgpu',
	diff_mode: 'normal',
	diff_whitespace: 'none',
	diff_view_mode: 'unified',
	theme: 'auto',
	font_size: FONT_SIZE_DEFAULT,
	high_contrast: false,
	arrow_gap_threshold: 100,
	language: 'en'
};

export const theme = writable<'dark' | 'light' | 'auto'>(DEFAULTS.theme);
export const diffMode = writable<'normal' | 'word-diff' | 'stat-only'>(DEFAULTS.diff_mode);
export const diffWhitespace = writable<
	'none' | 'ignore-space-change' | 'ignore-all-space' | 'ignore-blank-lines'
>(DEFAULTS.diff_whitespace);
export const diffViewMode = writable<'unified' | 'side-by-side'>(DEFAULTS.diff_view_mode);
export const renderer = writable<'wgpu' | 'canvas2d'>('wgpu');
export const fontSize = writable(DEFAULTS.font_size);
export const highContrast = writable(DEFAULTS.high_contrast);

function detectSystemTheme(): 'dark' | 'light' {
	if (typeof window === 'undefined') return 'dark';
	try {
		return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
	} catch {
		return 'dark';
	}
}

export const systemTheme = writable<'dark' | 'light'>(detectSystemTheme());

if (typeof window !== 'undefined') {
	try {
		const mql = window.matchMedia('(prefers-color-scheme: dark)');
		mql.addEventListener('change', (e) => {
			systemTheme.set(e.matches ? 'dark' : 'light');
		});
	} catch {
		// matchMedia not available
	}
}

export const resolvedTheme = derived([theme, systemTheme], ([$theme, $systemTheme]) =>
	$theme === 'auto' ? $systemTheme : $theme
);

function detectSystemHighContrast(): boolean {
	if (typeof window === 'undefined') return false;
	try {
		return window.matchMedia('(prefers-contrast: more)').matches;
	} catch {
		return false;
	}
}

export const systemHighContrast = writable(detectSystemHighContrast());

if (typeof window !== 'undefined') {
	try {
		const mql = window.matchMedia('(prefers-contrast: more)');
		mql.addEventListener('change', (e) => {
			systemHighContrast.set(e.matches);
		});
	} catch {
		// matchMedia not available
	}
}

export const resolvedHighContrast = derived(
	[highContrast, systemHighContrast],
	([$highContrast, $systemHighContrast]) => $highContrast || $systemHighContrast
);
let saveTimer: ReturnType<typeof setTimeout> | null = null;

function toPreferences(): AppPreferences {
	return {
		graph_color_mode: get(graphColorMode),
		graph_hide_merges: get(graphHideMerges),
		graph_orientation: get(graphOrientation),
		graph_palette: get(graphPalette),
		renderer: get(renderer),
		diff_mode: get(diffMode),
		diff_whitespace: get(diffWhitespace),
		diff_view_mode: get(diffViewMode),
		theme: get(theme),
		font_size: get(fontSize),
		high_contrast: get(highContrast),
		arrow_gap_threshold: get(arrowGapThreshold),
		language: get(locale)
	};
}

function updateFromPreferences(p: AppPreferences) {
	theme.set(p.theme);
	highContrast.set(p.high_contrast);
	if (p.renderer === 'wgpu' || p.renderer === 'canvas2d') {
		renderer.set(p.renderer);
	}
	if (p.font_size >= FONT_SIZE_MIN && p.font_size <= FONT_SIZE_MAX) {
		fontSize.set(p.font_size);
	}
	diffMode.set(p.diff_mode);
	diffWhitespace.set(p.diff_whitespace);
	if (p.diff_view_mode === 'unified' || p.diff_view_mode === 'side-by-side') {
		diffViewMode.set(p.diff_view_mode);
	}
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
	if (SUPPORTED_LOCALES.includes(p.language)) {
		setLocale(p.language);
	} else {
		setLocale(DEFAULT_LOCALE);
	}
	if (typeof p.arrow_gap_threshold === 'number' && p.arrow_gap_threshold >= 30) {
		arrowGapThreshold.set(p.arrow_gap_threshold);
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
	initLocale();
	try {
		const prefs = await getPreferences();
		updateFromPreferences(prefs);
	} catch {
		theme.set('auto');
	}
}

export function savePreferences() {
	const prefs = toPreferences();
	debouncedSave(prefs);
}

/**
 * Adjust the root font size by `delta` px (clamped to FONT_SIZE_MIN..FONT_SIZE_MAX).
 * Pass 0 to reset to the default. Persists the result and returns the new size.
 */
export function adjustFontSize(delta: number): number {
	const current = get(fontSize);
	const next =
		delta === 0
			? FONT_SIZE_DEFAULT
			: Math.max(FONT_SIZE_MIN, Math.min(FONT_SIZE_MAX, current + delta));
	fontSize.set(next);
	savePreferences();
	return next;
}
