import { writable } from 'svelte/store';

export type ToastSeverity = 'info' | 'warning' | 'error';

export interface Toast {
	id: number;
	message: string;
	severity: ToastSeverity;
	createdAt: number;
}

let nextId = 0;

export const toasts = writable<Toast[]>([]);

const toastTimers = new Map<number, ReturnType<typeof setTimeout>>();

const AUTO_DISMISS_MS: Record<ToastSeverity, number | null> = {
	info: 3000,
	warning: 5000,
	error: null
};

function scheduleDismiss(id: number, severity: ToastSeverity) {
	const dismissAfter = AUTO_DISMISS_MS[severity];
	if (dismissAfter !== null) {
		const timer = setTimeout(() => dismissToast(id), dismissAfter);
		toastTimers.set(id, timer);
	}
}

function clearTimer(id: number) {
	const timer = toastTimers.get(id);
	if (timer) {
		clearTimeout(timer);
		toastTimers.delete(id);
	}
}

export function showToast(message: string, severity: ToastSeverity = 'info'): number {
	const id = nextId++;
	const toast: Toast = { id, message, severity, createdAt: Date.now() };
	toasts.update((list) => [...list, toast]);
	scheduleDismiss(id, severity);
	return id;
}

export function updateToast(id: number, message: string, severity: ToastSeverity = 'info') {
	clearTimer(id);
	toasts.update((list) =>
		list.map((t) => (t.id === id ? { ...t, message, severity, createdAt: Date.now() } : t))
	);
	scheduleDismiss(id, severity);
}

export function dismissToast(id: number) {
	clearTimer(id);
	toasts.update((list) => list.filter((t) => t.id !== id));
}
