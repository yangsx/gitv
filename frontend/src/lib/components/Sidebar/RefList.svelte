<script lang="ts">
	import type { Ref } from '$lib/bindings/types';

	let {
		refs,
		onbranchselect
	}: {
		refs: Ref[];
		onbranchselect?: (name: string) => void;
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
			.sort((a, b) => a.name.localeCompare(b.name))
	);

	let remotes = $derived(
		refs
			.filter((r) => 'Remote' in r)
			.map((r) => r.Remote!)
			.sort((a, b) => a.name.localeCompare(b.name))
	);
</script>

<div class="space-y-3">
	{#if branches.length > 0}
		<div>
			<div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">
				Branches
			</div>
			<div class="space-y-0.5">
				{#each branches as branch (branch.name)}
					<button
						class="flex w-full items-center gap-1 rounded px-1.5 py-0.5 text-left hover:bg-gray-800"
						aria-label="Branch {branch.name}{branch.is_head ? ' (current)' : ''}"
						onclick={() => onbranchselect?.(branch.name)}
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
					</button>
				{/each}
			</div>
		</div>
	{/if}

	{#if tags.length > 0}
		<div>
			<div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">Tags</div>
			<div class="space-y-0.5">
				{#each tags as tag (tag.name)}
					<div class="flex items-center gap-1 px-1.5 py-0.5">
						<svg
							class="h-3 w-3 text-blue-400"
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
						<span class="truncate text-gray-300">{tag.name}</span>
					</div>
				{/each}
			</div>
		</div>
	{/if}

	{#if remotes.length > 0}
		<div>
			<div class="mb-1 text-[10px] font-semibold uppercase tracking-wider text-gray-500">
				Remotes
			</div>
			<div class="space-y-0.5">
				{#each remotes as remote (remote.remote + '/' + remote.name)}
					<div class="flex items-center gap-1 px-1.5 py-0.5">
						<span class="text-gray-500">{remote.remote}/</span>
						<span class="truncate text-gray-300">{remote.name}</span>
					</div>
				{/each}
			</div>
		</div>
	{/if}
</div>
