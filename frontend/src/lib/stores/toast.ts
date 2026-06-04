export type ToastSeverity = 'info' | 'warning' | 'error';

export interface Toast {
	id: number;
	message: string;
	severity: ToastSeverity;
	createdAt: number;
}

let nextId = 0;
let toasts = $state<Toast[]>([]);

const AUTO_DISMISS_MS: Record<ToastSeverity, number | null> = {
	info: 3000,
	warning: 5000,
	error: null
};

export function getToasts(): Toast[] {
	return toasts;
}

export function showToast(message: string, severity: ToastSeverity = 'info') {
	const id = nextId++;
	const toast: Toast = { id, message, severity, createdAt: Date.now() };
	toasts = [...toasts, toast];

	const dismissAfter = AUTO_DISMISS_MS[severity];
	if (dismissAfter !== null) {
		setTimeout(() => dismissToast(id), dismissAfter);
	}
}

export function dismissToast(id: number) {
	toasts = toasts.filter((t) => t.id !== id);
}
