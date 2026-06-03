import { writable } from 'svelte/store';
import type { RepositoryInfo } from '$lib/bindings/types';

export const repoInfo = writable<RepositoryInfo | null>(null);
export const selectedOid = writable<string | null>(null);
export const isLoading = writable(false);
export const error = writable<string | null>(null);
