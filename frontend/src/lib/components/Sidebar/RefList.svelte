<script lang="ts">
	import type { Ref } from '$lib/bindings/types';
	import { t } from '$lib/stores/locale';
	import { parseSemver, compareSemverDesc } from '$lib/semver';

	let {
		refs,
		onbranchselect,
		onbranchcontextmenu,
		ontagselect,
		onremoteselect,
		onremotecontextmenu,
		selectedBranch,
		selectedRemote,
		selectedTag
	}: {
		refs: Ref[];
		onbranchselect?: (_name: string) => void;
		onbranchcontextmenu?: (_e: MouseEvent, _name: string) => void;
		ontagselect?: (_name: string) => void;
		onremoteselect?: (_remote: string, _name: string) => void;
		onremotecontextmenu?: (_e: MouseEvent, _remote: string, _name: string) => void;
		selectedBranch?: string | null;
		selectedRemote?: string | null;
		selectedTag?: string | null;
	} = $props();

	let branches = $derived(
		refs
			.filter((r) => 'Branch' in r)
			.map((r) => r.Branch!)
			.sort((a, b) => (a.is_head === b.is_head ? a.name.localeCompare(b.name) : a.is_head ? -1 : 1))
	);

	let tags = $derived(
		refs
			.filter((r) => 'Tag' in r)
			.map((r) => r.Tag!)
			.sort((a, b) => {
				const va = parseSemver(a.name);
				const vb = parseSemver(b.name);
				if (va && vb) return compareSemverDesc(va, vb);
				if (va) return -1;
				if (vb) return 1;
				return a.name.localeCompare(b.name);
			})
	);

	let remotes = $derived(
		refs
			.filter((r) => 'Remote' in r)
			.map((r) => r.Remote!)
			.sort((a, b) => a.name.localeCompare(b.name))
	);
</script>

{#if branches.length === 0 && tags.length === 0 && remotes.length === 0}
	<div class="py-2 text-xs italic text-gray-500">{$t('sidebar.no_refs')}</div>
{:else}
	<div class="space-y-3">
		{#if branches.length > 0}
			<div>
				<div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">
					{$t('sidebar.branches')}
				</div>
				<div class="space-y-0.5">
					{#each branches as branch (branch.name)}
						<button
							class="flex w-full items-center gap-1 rounded px-1.5 py-0.5 text-left hover:bg-gray-800 {branch.name ===
							selectedBranch
								? 'bg-blue-900/40 text-blue-300'
								: branch.is_merged
									? 'text-gray-500'
									: ''}"
							aria-label={branch.is_head
								? $t('sidebar.branch_aria', { name: branch.name })
								: $t('sidebar.branch_aria_default', { name: branch.name })}
							onclick={() => onbranchselect?.(branch.name)}
							oncontextmenu={(e: MouseEvent) => {
								e.preventDefault();
								onbranchcontextmenu?.(e, branch.name);
							}}
						>
							{#if branch.is_head}
								<span class="text-green-400" aria-hidden="true">*</span>
							{:else}
								<svg
									class="h-3 w-3 text-gray-500"
									fill="none"
									viewBox="0 0 24 24"
									stroke="currentColor"
									aria-hidden="true"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										stroke-width="2"
										d="M13 10V3L4 14h7v7l9-11h-7z"
									/>
								</svg>
							{/if}
							<span class="truncate text-gray-300">{branch.name}</span>
							<span class="ml-auto flex gap-1 shrink-0">
								{#if branch.ahead > 0}
									<span class="text-[10px] text-green-500 font-mono" title="ahead {branch.ahead}"
										>↑{branch.ahead}</span
									>
								{/if}
								{#if branch.behind > 0}
									<span class="text-[10px] text-red-500 font-mono" title="behind {branch.behind}"
										>↓{branch.behind}</span
									>
								{/if}
							</span>
						</button>
					{/each}
				</div>
			</div>
		{/if}

		{#if tags.length > 0}
			<div>
				<div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">
					{$t('sidebar.tags')}
				</div>
				<div class="space-y-0.5">
					{#each tags as tag (tag.name)}
						<button
							class="flex w-full items-center gap-1 rounded px-1.5 py-0.5 text-left hover:bg-gray-800 {tag.name ===
							selectedTag
								? 'bg-blue-900/40 text-blue-300'
								: 'text-gray-300'}"
							aria-label={$t('sidebar.tag_aria', { name: tag.name })}
							onclick={() => ontagselect?.(tag.name)}
						>
							<svg
								class="h-3 w-3 shrink-0 text-blue-400"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"
								/>
							</svg>
							<span class="truncate">{tag.name}</span>
						</button>
					{/each}
				</div>
			</div>
		{/if}

		{#if remotes.length > 0}
			<div>
				<div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">
					{$t('sidebar.remotes')}
				</div>
				<div class="space-y-0.5">
					{#each remotes as remote (remote.remote + '/' + remote.name)}
						<button
							class="flex w-full items-center gap-1 rounded px-1.5 py-0.5 text-left hover:bg-gray-800 {remote.remote +
								'/' +
								remote.name ===
							selectedRemote
								? 'bg-blue-900/40 text-blue-300'
								: ''}"
							aria-label={$t('sidebar.remote_aria', {
								remote: remote.remote,
								name: remote.name
							})}
							onclick={() => onremoteselect?.(remote.remote, remote.name)}
							oncontextmenu={(e: MouseEvent) => {
								e.preventDefault();
								onremotecontextmenu?.(e, remote.remote, remote.name);
							}}
						>
							<svg
								class="h-3 w-3 shrink-0 text-gray-500"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
								aria-hidden="true"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M3.055 11H5a2 2 0 012 2v1a2 2 0 002 2 2 2 0 012 2v2.945M8 3.935V5.5A2.5 2.5 0 0010.5 8h.5a2 2 0 012 2 2 2 0 104 0 2 2 0 012-2h1.064M15 20.488V18a2 2 0 012-2h3.064M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
								/>
							</svg>
							<span class="text-gray-500">{remote.remote}/</span>
							<span class="truncate text-gray-300">{remote.name}</span>
						</button>
					{/each}
				</div>
			</div>
		{/if}
	</div>
{/if}
