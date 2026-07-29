<script lang="ts">
	import type { RepoPathValidation, PathSuggestion } from '$lib/bindings/types';
	import { t } from '$lib/stores/locale';

	interface Props {
		repoInput: string;
		authToken: string;
		showRemoteAuth: boolean;
		validation: RepoPathValidation | null;
		pathSuggestions: PathSuggestion[];
		showSuggestions: boolean;
		suggestionIndex: number;
		onSubmit: () => void;
		onBrowse: () => void;
		onInputChange: () => void;
		acceptSuggestion: (_path: string) => void;
		showAuthButton?: boolean;
	}

	let {
		repoInput = $bindable(''),
		authToken = $bindable(''),
		showRemoteAuth = $bindable(false),
		validation = $bindable(null as RepoPathValidation | null),
		pathSuggestions = $bindable([] as PathSuggestion[]),
		showSuggestions = $bindable(false),
		suggestionIndex = $bindable(-1),
		onSubmit,
		onBrowse,
		onInputChange,
		acceptSuggestion,
		showAuthButton = true
	}: Props = $props();
</script>

<div class="flex gap-1.5">
	<div class="relative flex-1">
		<input
			type="text"
			class="w-full rounded bg-gray-800 py-2 pl-3 pr-8 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
			placeholder={$t('page.repo_input_placeholder')}
			bind:value={repoInput}
			oninput={onInputChange}
			onkeydown={(e) => {
				if (showSuggestions && pathSuggestions.length > 0) {
					if (e.key === 'ArrowDown') {
						e.preventDefault();
						suggestionIndex = Math.min(suggestionIndex + 1, pathSuggestions.length - 1);
					} else if (e.key === 'ArrowUp') {
						e.preventDefault();
						suggestionIndex = Math.max(suggestionIndex - 1, 0);
					} else if (e.key === 'Enter' && suggestionIndex >= 0) {
						e.preventDefault();
						acceptSuggestion(pathSuggestions[suggestionIndex].path);
					} else if (e.key === 'Escape') {
						e.preventDefault();
						showSuggestions = false;
						pathSuggestions = [];
						suggestionIndex = -1;
					}
					return;
				}
				if (e.key === 'Enter' && repoInput.trim()) {
					onSubmit();
				}
			}}
		/>
		{#if validation}
			<span
				class="absolute right-2 top-1/2 -translate-y-1/2 text-xs {validation.valid
					? 'text-green-500'
					: 'text-amber-500'}"
			>
				{validation.kind === 'remote' ? '🌐' : validation.valid ? '✓' : '!'}
			</span>
		{/if}
	</div>
	<button
		class="flex-shrink-0 rounded bg-gray-700 p-2 text-gray-400 hover:bg-gray-600 hover:text-white"
		onclick={onBrowse}
		title={$t('page.browse_repo')}
		aria-label={$t('page.browse_repo')}
	>
		<svg class="h-4 w-4" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"
			><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" /></svg
		>
	</button>
</div>
{#if validation?.message}
	<p class="text-xs {validation.valid ? 'text-green-500' : 'text-amber-500'}">
		{validation.message}
	</p>
{/if}
{#if showSuggestions && pathSuggestions.length > 0}
	<div class="max-h-40 overflow-y-auto rounded bg-gray-800 border border-gray-700">
		{#each pathSuggestions as s, i (s.path)}
			<button
				class="block w-full px-3 py-1.5 text-left text-sm text-gray-300 hover:bg-gray-700 {i ===
				suggestionIndex
					? 'bg-gray-700'
					: ''}"
				onclick={() => acceptSuggestion(s.path)}
			>
				{s.path}
			</button>
		{/each}
	</div>
{/if}
{#if showRemoteAuth}
	<div class="flex gap-1.5">
		<input
			type="password"
			class="flex-1 rounded bg-gray-800 px-3 py-2 text-sm text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
			placeholder={$t('page.auth_token_placeholder')}
			bind:value={authToken}
			onkeydown={(e) => {
				if (e.key === 'Enter') onSubmit();
			}}
		/>
		{#if showAuthButton}
			<button
				class="rounded bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-40"
				onclick={onSubmit}
				disabled={!authToken.trim()}
			>
				{$t('page.authenticate')}
			</button>
		{/if}
	</div>
{/if}
