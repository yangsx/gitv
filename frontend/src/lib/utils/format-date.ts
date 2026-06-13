import { get } from 'svelte/store';
import { locale } from '$lib/stores/locale';

export function formatDate(iso: string, options?: Intl.DateTimeFormatOptions): string {
	try {
		return new Date(iso).toLocaleDateString(get(locale), {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			...options
		});
	} catch {
		return '';
	}
}

export function formatDateTime(iso: string): string {
	try {
		return new Date(iso).toLocaleString(get(locale));
	} catch {
		return '';
	}
}

export function formatDateTimeShort(iso: string): string {
	try {
		const d = new Date(iso);
		return (
			d.toLocaleDateString(get(locale)) +
			' ' +
			d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
		);
	} catch {
		return '';
	}
}
