import { invoke } from '@tauri-apps/api/core';
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

export async function openRepository(path: string): Promise<RepositoryInfo> {
	return invoke<RepositoryInfo>('open_repository', { path });
}

export async function getRefs(path: string): Promise<Ref[]> {
	return invoke<Ref[]>('get_refs', { path });
}

export async function getRecentRepositories(): Promise<RecentRepository[]> {
	return invoke<RecentRepository[]>('get_recent_repositories');
}

export async function getCommits(path: string): Promise<CommitInfo[]> {
	return invoke<CommitInfo[]>('get_commits', { path, filter: null });
}

export async function getGraphLayout(
	path: string,
	options?: {
		hide_merges?: boolean;
		orientation?: string;
		color_mode?: string;
	}
): Promise<GraphLayout> {
	return invoke<GraphLayout>('get_graph_layout', {
		path,
		hide_merges: options?.hide_merges ?? false,
		orientation: options?.orientation ?? 'top-to-bottom',
		color_mode: options?.color_mode ?? 'by-branch'
	});
}

export async function searchCommits(path: string, query: SearchQuery): Promise<SearchResult[]> {
	return invoke<SearchResult[]>('search_commits', { path, query });
}

export async function getCommitDetails(path: string, oid: string): Promise<CommitDetails> {
	return invoke<CommitDetails>('get_commit_details', { path, oid });
}

export async function getDiff(
	path: string,
	from: string | null,
	to: string,
	whitespace?: string
): Promise<DiffSummary> {
	return invoke<DiffSummary>('get_diff', { path, from, to, whitespace: whitespace ?? null });
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
	return invoke<FileDiff>('get_file_diff', {
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
	return invoke<FileTreeNode>('get_file_tree', { path, atCommit: atCommit ?? null });
}

export async function getFileHistory(
	path: string,
	filePath: string,
	maxCount?: number
): Promise<FileHistoryEntry[]> {
	return invoke<FileHistoryEntry[]>('get_file_history', {
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
	return invoke<string>('get_blob_content', { path, atCommit, filePath });
}

export async function getReflog(path: string, refName?: string): Promise<ReflogEntry[]> {
	return invoke<ReflogEntry[]>('get_reflog', { path, refName: refName ?? null });
}

export async function getStashList(path: string): Promise<StashEntry[]> {
	return invoke<StashEntry[]>('get_stash_list', { path });
}

export async function getStashDiff(path: string, stashIndex: number): Promise<FileDiff> {
	return invoke<FileDiff>('get_stash_diff', { path, stashIndex });
}

export async function getStashSplitDiff(path: string, stashIndex: number): Promise<StashSplitDiff> {
	return invoke<StashSplitDiff>('get_stash_split_diff', { path, stashIndex });
}

export async function getBlame(path: string, filePath: string, atCommit?: string): Promise<Blame> {
	return invoke<Blame>('get_blame', { path, filePath, atCommit: atCommit ?? null });
}

export async function saveSearch(path: string, name: string, query: string): Promise<SavedSearch> {
	return invoke<SavedSearch>('save_search', { repoPath: path, name, query });
}

export async function listSavedSearches(path: string): Promise<SavedSearch[]> {
	return invoke<SavedSearch[]>('list_saved_searches', { repoPath: path });
}

export async function deleteSavedSearch(path: string, id: string): Promise<void> {
	return invoke<void>('delete_saved_search', { repoPath: path, id });
}

export async function getWorkingChanges(path: string): Promise<WorkingChangesDiff> {
	return invoke<WorkingChangesDiff>('get_working_changes', { path });
}

export async function getWorkingChangesDiffs(
	path: string,
	staged: boolean,
	diffMode?: string,
	whitespace?: string
): Promise<FileDiff[]> {
	return invoke<FileDiff[]>('get_working_changes_diffs', {
		path,
		staged,
		diffMode: diffMode ?? null,
		whitespace: whitespace ?? null
	});
}
