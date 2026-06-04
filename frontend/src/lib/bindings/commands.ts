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
	SearchQuery,
	SearchResult
} from './types';

export async function openRepository(path: string): Promise<RepositoryInfo> {
	return invoke<RepositoryInfo>('open_repository', { path });
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
	whitespace?: string
): Promise<FileDiff> {
	return invoke<FileDiff>('get_file_diff', {
		path,
		from,
		to,
		filePath,
		diffMode: diffMode ?? null,
		whitespace: whitespace ?? null
	});
}

export async function getFileTree(path: string, atCommit?: string | null): Promise<FileTreeNode> {
	return invoke<FileTreeNode>('get_file_tree', { path, atCommit: atCommit ?? null });
}
