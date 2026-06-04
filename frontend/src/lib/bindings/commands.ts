// eslint-disable-next-line no-unused-vars, @typescript-eslint/no-unused-vars
import { invoke } from '@tauri-apps/api/core';
import { recordIpcTiming } from '$lib/stores/debug';
import type {
	GraphLayout,
	RepositoryInfo,
	RecentRepository,
	CommitInfo,
	CommitDetails,
	DiffSummary,
	FileDiff,
	FileTreeNode,
	FileHistoryEntry,
	SearchQuery,
	SearchResult,
	Ref,
	ReflogEntry,
	StashEntry,
	StashSplitDiff,
	Blame,
	SavedSearch,
	WorkingChangesDiff
} from './types';

async function timedInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
	const start = performance.now();
	try {
		const result = await timedInvoke<T>(command, args);
		return result;
	} finally {
		recordIpcTiming(command, performance.now() - start);
	}
}

export async function openRepository(path: string): Promise<RepositoryInfo> {
	return timedInvoke<RepositoryInfo>('open_repository', { path });
}

export async function getRefs(path: string): Promise<Ref[]> {
	return timedInvoke<Ref[]>('get_refs', { path });
}

export async function getRecentRepositories(): Promise<RecentRepository[]> {
	return timedInvoke<RecentRepository[]>('get_recent_repositories');
}

export async function getCommits(path: string): Promise<CommitInfo[]> {
	return timedInvoke<CommitInfo[]>('get_commits', { path, filter: null });
}

export async function getGraphLayout(
	path: string,
	options?: {
		hide_merges?: boolean;
		orientation?: string;
		color_mode?: string;
		palette?: string;
	}
): Promise<GraphLayout> {
	return timedInvoke<GraphLayout>('get_graph_layout', {
		path,
		hide_merges: options?.hide_merges ?? false,
		orientation: options?.orientation ?? 'top-to-bottom',
		color_mode: options?.color_mode ?? 'by-branch',
		palette: options?.palette ?? null
	});
}

export async function searchCommits(path: string, query: SearchQuery): Promise<SearchResult[]> {
	return timedInvoke<SearchResult[]>('search_commits', { path, query });
}

export async function getCommitDetails(path: string, oid: string): Promise<CommitDetails> {
	return timedInvoke<CommitDetails>('get_commit_details', { path, oid });
}

export async function getDiff(
	path: string,
	from: string | null,
	to: string,
	whitespace?: string
): Promise<DiffSummary> {
	return timedInvoke<DiffSummary>('get_diff', { path, from, to, whitespace: whitespace ?? null });
}

export async function getFileDiff(
	path: string,
	from: string | null,
	to: string,
	filePath: string,
	diffMode?: string,
	whitespace?: string,
	full?: boolean
): Promise<FileDiff> {
	return timedInvoke<FileDiff>('get_file_diff', {
		path,
		from,
		to,
		filePath,
		diffMode: diffMode ?? null,
		whitespace: whitespace ?? null,
		full: full ?? false
	});
}

export async function getFileTree(path: string, atCommit?: string | null): Promise<FileTreeNode> {
	return timedInvoke<FileTreeNode>('get_file_tree', { path, atCommit: atCommit ?? null });
}

export async function getFileHistory(
	path: string,
	filePath: string,
	maxCount?: number
): Promise<FileHistoryEntry[]> {
	return timedInvoke<FileHistoryEntry[]>('get_file_history', {
		path,
		filePath,
		maxCount: maxCount ?? null
	});
}

export async function getBlobContent(
	path: string,
	atCommit: string,
	filePath: string
): Promise<string> {
	return timedInvoke<string>('get_blob_content', { path, atCommit, filePath });
}

export async function getReflog(path: string, refName?: string): Promise<ReflogEntry[]> {
	return timedInvoke<ReflogEntry[]>('get_reflog', { path, refName: refName ?? null });
}

export async function getStashList(path: string): Promise<StashEntry[]> {
	return timedInvoke<StashEntry[]>('get_stash_list', { path });
}

export async function getStashDiff(path: string, stashIndex: number): Promise<FileDiff> {
	return timedInvoke<FileDiff>('get_stash_diff', { path, stashIndex });
}

export async function getStashSplitDiff(path: string, stashIndex: number): Promise<StashSplitDiff> {
	return timedInvoke<StashSplitDiff>('get_stash_split_diff', { path, stashIndex });
}

export async function getBlame(path: string, filePath: string, atCommit?: string): Promise<Blame> {
	return timedInvoke<Blame>('get_blame', { path, filePath, atCommit: atCommit ?? null });
}

export async function saveSearch(path: string, name: string, query: string): Promise<SavedSearch> {
	return timedInvoke<SavedSearch>('save_search', { repoPath: path, name, query });
}

export async function listSavedSearches(path: string): Promise<SavedSearch[]> {
	return timedInvoke<SavedSearch[]>('list_saved_searches', { repoPath: path });
}

export async function deleteSavedSearch(path: string, id: string): Promise<void> {
	return timedInvoke<void>('delete_saved_search', { repoPath: path, id });
}

export async function getWorkingChanges(path: string): Promise<WorkingChangesDiff> {
	return timedInvoke<WorkingChangesDiff>('get_working_changes', { path });
}

export async function getWorkingChangesDiffs(
	path: string,
	staged: boolean,
	diffMode?: string,
	whitespace?: string
): Promise<FileDiff[]> {
	return timedInvoke<FileDiff[]>('get_working_changes_diffs', {
		path,
		staged,
		diffMode: diffMode ?? null,
		whitespace: whitespace ?? null
	});
}
