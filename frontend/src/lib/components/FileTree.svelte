<script lang="ts">
	import type { FileTreeNode } from '$lib/bindings/types';

	interface Props {
		node: FileTreeNode;
		repoPath: string;
		depth?: number;
		onhistoryfile?: (_path: string) => void;
		onselectfile?: (_path: string) => void;
	}

	let { node, repoPath, depth = 0, onhistoryfile, onselectfile }: Props = $props();
	// svelte-ignore state_referenced_locally
	let expanded = $state(depth < 1);

	function toggle() {
		if (node.children.length > 0) {
			expanded = !expanded;
		} else if (node.node_type === 'File' && onselectfile) {
			onselectfile(node.path);
		}
	}

	function handleContextmenu(e: Event) {
		if (node.node_type === 'File' && onhistoryfile) {
			e.preventDefault();
			onhistoryfile(node.path);
		}
	}

	function nodeIcon(node: FileTreeNode): string {
		switch (node.node_type) {
			case 'Directory':
				return expanded ? '\u{1F4C2}' : '\u{1F4C1}';
			case 'Symlink':
				return '\u{1F517}';
			case 'Submodule':
				return '\u{1F4E6}';
			default:
				return '\u{1F4C4}';
		}
	}
</script>

{#if node.name}
	<button
		class="flex w-full items-center gap-1.5 border-b border-gray-800/50 px-3 py-1 text-left text-xs hover:bg-gray-800/70"
		style="padding-left: {12 + depth * 16}px;"
		onclick={toggle}
		oncontextmenu={handleContextmenu}
	>
		<span class="shrink-0 text-sm">{nodeIcon(node)}</span>
		<span class="flex-1 truncate font-mono text-gray-300">{node.name}</span>
		{#if node.size !== null && node.node_type === 'File'}
			<span class="shrink-0 text-[10px] text-gray-500"
				>{node.size > 1024 ? (node.size / 1024).toFixed(1) + 'K' : node.size + 'B'}</span
			>
		{/if}
	</button>
{/if}

{#if expanded}
	{#each node.children as child (child.path)}
		<!-- svelte-ignore svelte_self_deprecated -->
		<svelte:self node={child} {repoPath} depth={depth + 1} {onhistoryfile} {onselectfile} />
	{/each}
{/if}
