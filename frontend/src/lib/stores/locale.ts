import { writable, derived, get } from 'svelte/store';
import en from '$lib/locales/en.json';
import zhCN from '$lib/locales/zh-CN.json';

export type Locale = 'en' | 'zh-cn';

type TranslationDict = {
	[key: string]: string | TranslationDict;
};

const translations: Record<Locale, TranslationDict> = {
	en,
	'zh-cn': zhCN
};

function lookup(dict: TranslationDict, key: string): string {
	const parts = key.split('.');
	let obj: TranslationDict | string | undefined = dict;
	for (const part of parts) {
		if (typeof obj !== 'object' || obj === null) return key;
		obj = (obj as TranslationDict)[part];
	}
	return typeof obj === 'string' ? obj : key;
}

export const locale = writable<Locale>('en');

export function initLocale(preferred?: Locale): Locale {
	const detected = detectSystemLocale();
	const lang = preferred ?? detected;
	if (translations[lang]) {
		locale.set(lang);
		return lang;
	}
	locale.set('en');
	return 'en';
}

export function setLocale(lang: Locale) {
	if (translations[lang]) {
		locale.set(lang);
	}
}

function detectSystemLocale(): Locale {
	try {
		const navLang = navigator.language.toLowerCase();
		if (navLang.startsWith('zh')) return 'zh-cn';
		return 'en';
	} catch {
		return 'en';
	}
}

function interpolate(str: string, params?: Record<string, string | number>): string {
	if (!params) return str;
	return str.replace(/\{(\w+)\}/g, (_, key) => {
		const val = params[key];
		return val !== undefined ? String(val) : `{${key}}`;
	});
}

export const t = derived(locale, ($locale) => {
	const dict = translations[$locale] ?? translations.en;
	return (key: string, params?: Record<string, string | number>): string => {
		const str = lookup(dict, key);
		return interpolate(str, params);
	};
});

export function translate(key: string, params?: Record<string, string | number>): string {
	const dict = translations[get(locale)] ?? translations.en;
	const str = lookup(dict, key);
	return interpolate(str, params);
}
