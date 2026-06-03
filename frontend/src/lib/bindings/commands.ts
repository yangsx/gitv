import { invoke } from '@tauri-apps/api/core';
import type { GraphLayout, RepositoryInfo, RecentRepository } from './types';

export async function openRepository(path: string): Promise<RepositoryInfo> {
	return invoke<RepositoryInfo>('open_repository', { path });
}

export async function getRecentRepositories(): Promise<RecentRepository[]> {
	return invoke<RecentRepository[]>('get_recent_repositories');
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
