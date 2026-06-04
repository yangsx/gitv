import { writable, derived } from 'svelte/store';
import type { RepositoryInfo, SearchQuery, SearchResult } from '$lib/bindings/types';

export const repoInfo = writable<RepositoryInfo | null>(null);
export const selectedOid = writable<string | null>(null);
export const comparisonOid = writable<string | null>(null);
export type OperationState =
	| 'Idle'
	| 'LoadingRepo'
	| 'LoadingDetails'
	| 'Searching'
	| 'ApplyingFilter';

export const operationState = writable<OperationState>('Idle');

export const isLoading = derived(operationState, (s) => s !== 'Idle');
export const error = writable<string | null>(null);
export const searchQuery = writable<SearchQuery | null>(null);
export const searchResults = writable<SearchResult[]>([]);

export const graphColorMode = writable<'by-branch' | 'by-author'>('by-branch');
export const graphHideMerges = writable(false);
export const graphOrientation = writable<'top-to-bottom' | 'bottom-to-top'>('top-to-bottom');
export const graphPalette = writable<'default' | 'deuteranopia' | 'protanopia' | 'tritanopia'>(
	'default'
);

export const matchingOids = derived(
	searchResults,
	($results) => new Set($results.map((r) => r.commit_oid))
);
