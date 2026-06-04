import { writable, derived } from 'svelte/store';
import type { RepositoryInfo, SearchQuery, SearchResult } from '$lib/bindings/types';

export const repoInfo = writable<RepositoryInfo | null>(null);
export const selectedOid = writable<string | null>(null);
export const comparisonOid = writable<string | null>(null);
export const isLoading = writable(false);
export const error = writable<string | null>(null);
export const searchQuery = writable<SearchQuery | null>(null);
export const searchResults = writable<SearchResult[]>([]);

export const matchingOids = derived(
	searchResults,
	($results) => new Set($results.map((r) => r.commit_oid))
);
