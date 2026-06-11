import { writable, derived, get } from 'svelte/store';

type TranslationDict = {
	[key: string]: string | TranslationDict;
};

type LocaleModule = { default: TranslationDict };

const localeModules = import.meta.glob<LocaleModule>('/src/lib/locales/*.json', {
	eager: true
});

function extractLocaleCode(path: string): string {
	const filename = path.split('/').pop() ?? '';
	const stem = filename.replace(/\.json$/, '');
	return stem.toLowerCase();
}

export const SUPPORTED_LOCALES: string[] = Object.keys(localeModules)
	.map(extractLocaleCode)
	.sort((a, b) => a.localeCompare(b));

export const DEFAULT_LOCALE = 'en';

const translations: Record<string, TranslationDict> = {};
for (const [path, mod] of Object.entries(localeModules)) {
	const code = extractLocaleCode(path);
	translations[code] = (mod as LocaleModule).default;
}

function lookup(dict: TranslationDict, key: string): string {
	const parts = key.split('.');
	let obj: TranslationDict | string | undefined = dict;
	for (const part of parts) {
		if (typeof obj !== 'object' || obj === null) return key;
		obj = (obj as TranslationDict)[part];
	}
	return typeof obj === 'string' ? obj : key;
}

export const locale = writable<string>(DEFAULT_LOCALE);

function matchSystemLocale(): string {
	try {
		const navLang = navigator.language.toLowerCase();
		if (translations[navLang]) return navLang;
		const prefix = navLang.split('-')[0];
		const match = SUPPORTED_LOCALES.find((l) => l.startsWith(prefix));
		if (match) return match;
	} catch {
		// fall through
	}
	return DEFAULT_LOCALE;
}

export function initLocale(preferred?: string): string {
	const lang = preferred ?? matchSystemLocale();
	if (translations[lang]) {
		locale.set(lang);
		return lang;
	}
	locale.set(DEFAULT_LOCALE);
	return DEFAULT_LOCALE;
}

export function setLocale(lang: string) {
	if (translations[lang]) {
		locale.set(lang);
	}
}

export function getLocaleSelfName(lang: string): string {
	const dict = translations[lang];
	if (!dict) return lang;
	const selfName = lookup(dict, 'lang_self');
	return selfName === 'lang_self' ? lang : selfName;
}

function interpolate(str: string, params?: Record<string, string | number>): string {
	if (!params) return str;
	return str.replace(/\{(\w+)\}/g, (_, key) => {
		const val = params[key];
		return val !== undefined ? String(val) : `{${key}}`;
	});
}

export const t = derived(locale, ($locale) => {
	const dict = translations[$locale] ?? translations[DEFAULT_LOCALE];
	return (key: string, params?: Record<string, string | number>): string => {
		const str = lookup(dict, key);
		return interpolate(str, params);
	};
});

export function translate(key: string, params?: Record<string, string | number>): string {
	const dict = translations[get(locale)] ?? translations[DEFAULT_LOCALE];
	const str = lookup(dict, key);
	return interpolate(str, params);
}
