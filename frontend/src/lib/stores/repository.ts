import { writable, derived } from 'svelte/store';
import type { RepositoryInfo, CommitInfo, GraphLayout } from '$lib/bindings/types';

export const repoInfo = writable<RepositoryInfo | null>(null);
export const commits = writable<CommitInfo[]>([]);
export const graphLayout = writable<GraphLayout | null>(null);
export const selectedOid = writable<string | null>(null);
export const isLoading = writable(false);
export const error = writable<string | null>(null);

export const selectedCommit = derived(
	[commits, selectedOid],
	([$commits, $oid]) => $commits.find((c) => c.oid === $oid) ?? null
);
