import { invoke } from '@tauri-apps/api/core';

let patched = false;

export function initLogging() {
	if (patched) return;
	patched = true;

	const origWarn = console.warn;
	const origError = console.error;

	console.warn = (...args: unknown[]) => {
		origWarn.apply(console, args);
		const msg = args.map(String).join(' ');
		invoke('log_frontend_message', { level: 'warn', message: msg }).catch(() => {});
	};

	console.error = (...args: unknown[]) => {
		origError.apply(console, args);
		const msg = args.map(String).join(' ');
		invoke('log_frontend_message', { level: 'error', message: msg }).catch(() => {});
	};
}
