<script lang="ts">
	import { onMount } from 'svelte';
	import { untrack } from 'svelte';
	import { get } from 'svelte/store';
	import {
		getGraphLayout,
		getCommitDetails,
		getInitialData,
		getWorkingChanges,
		getStartupInfo,
		getRecentRepositories,
		saveRecentRepository,
		openInNewWindow,
		quitApp
	} from '$lib/bindings/commands';
	import type {
		CommitInfo,
		GraphLayout,
		CommitDetails,
		Ref,
		WorkingChangesDiff,
		FileChange,
		RecentRepository
	} from '$lib/bindings/types';
	import {
		repoInfo,
		selectedOid,
		error,
		matchingOids,
		comparisonOid,
		graphColorMode,
		graphHideMerges,
		graphOrientation,
		graphPalette,
		operationState,
		sortBy,
		sortAsc,
		searchShowMode
	} from '$lib/stores/repository';
	import CommitList from '$lib/components/CommitList.svelte';
	import SearchBar from '$lib/components/SearchBar.svelte';
	import CommitDetailPanel from '$lib/components/CommitDetailPanel.svelte';
	import ComparisonPanel from '$lib/components/ComparisonPanel.svelte';
	import ResizeHandle from '$lib/components/ResizeHandle.svelte';
	import Sidebar from '$lib/components/Sidebar/Sidebar.svelte';
	import RefList from '$lib/components/Sidebar/RefList.svelte';
	import StashList from '$lib/components/Sidebar/StashList.svelte';
	import ReflogPanel from '$lib/components/Sidebar/ReflogPanel.svelte';
	import FileHistoryPanel from '$lib/components/FileHistoryPanel.svelte';
	import Toolbar from '$lib/components/Toolbar.svelte';
	import AuthorLegend from '$lib/components/AuthorLegend.svelte';
	import ToastContainer from '$lib/components/ToastContainer.svelte';
	import { showToast } from '$lib/stores/toast';
	import {
		toggleDebug,
		tickFps,
		updateDebugGraphStats,
		updateLoadPhaseTimings,
		debugOverlayEnabled,
		logPath,
		startMemoryTracking,
		stopMemoryTracking
	} from '$lib/stores/debug';
	import DebugOverlay from '$lib/components/DebugOverlay.svelte';
	import { getClampedLayout, updateLayout } from '$lib/stores/layout';
	import { registerCommand, unregisterCommandsByPrefix } from '$lib/stores/commands';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import type { ContextMenuItem } from '$lib/components/ContextMenu.svelte';
	import PreferencesModal from '$lib/components/PreferencesModal.svelte';
	import ShortcutHelp from '$lib/components/ShortcutHelp.svelte';
	import InfoDialog from '$lib/components/InfoDialog.svelte';
	import { initPreferences, theme, fontSize, highContrast } from '$lib/stores/preferences';
	import { t, translate, locale } from '$lib/stores/locale';
	import { computeHideMergeLayout } from '$lib/graph/hide-merges';

	let repoPath = $state('');
	let startupComplete = $state(false);
	let autoDialogShown = $state(false);
	let recentRepos = $state<RecentRepository[]>([]);
	let commits = $state<CommitInfo[]>([]);
	let graphLayout = $state<GraphLayout | null>(null);
	let layoutGeneration = 0;
	let commitDetails = $state<CommitDetails | null>(null);
	let detailsLoading = $state(false);
	let savedLayout = typeof window !== 'undefined' ? getClampedLayout() : null;
	let detailPanelHeight = $state(savedLayout?.detailPanelHeight ?? 400);
	let viewportHeight = $state(typeof window !== 'undefined' ? window.innerHeight : 800);
	let allRefs = $state<Ref[]>([]);
	let historyFilePath = $state<string | null>(null);
	let historyRevision = $state(0);
	let sidebarGotoTab = $state<'refs' | 'stash' | 'reflog' | 'history'>('refs');
	let workingChangesDiff = $state<WorkingChangesDiff | null>(null);
	let showCommandPalette = $state(false);
	let contextMenu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null);
	let isDragging = $state(false);
	let isFullscreen = $state(false);
	let selectedBranch = $state<string | null>(null);
	let selectedRemote = $state<string | null>(null);
	let selectedTag = $state<string | null>(null);
	let showPreferences = $state(false);
	let showShortcutHelp = $state(false);
	let showInfo = $state(false);

	let uncommittedCount = $derived(
		workingChangesDiff ? workingChangesDiff.staged.length + workingChangesDiff.unstaged.length : 0
	);
	let detachedHeadSha = $derived(
		$repoInfo?.head_commit && !$repoInfo?.head_branch ? $repoInfo.head_commit.substring(0, 7) : null
	);

	const ROW_HEIGHT_REM = 1.75;
	let rowHeight = $derived(Math.round(ROW_HEIGHT_REM * $fontSize));

	let branchNames = $derived(
		allRefs
			.filter((r) => 'Branch' in r)
			.map((r) => r.Branch!.name)
			.sort((a, b) => {
				const aIsHead = allRefs.some(
					(r) => 'Branch' in r && r.Branch?.name === a && r.Branch?.is_head
				);
				const bIsHead = allRefs.some(
					(r) => 'Branch' in r && r.Branch?.name === b && r.Branch?.is_head
				);
				if (aIsHead !== bIsHead) return aIsHead ? -1 : 1;
				return a.localeCompare(b);
			})
	);
	let sidebarWidth = $state(savedLayout?.sidebarWidth ?? 220);

	let focusBranchOid = $state<string | null>(null);
	let refreshSignal = $state(0);

	const STAGED_OID = '__staged__';
	const UNSTAGED_OID = '__unstaged__';
	const VIRTUAL_OIDS = new Set([STAGED_OID, UNSTAGED_OID]);

	onMount(() => {
		registerCommands();

		function doStartup() {
			const params = new URLSearchParams(window.location.search);
			const pathParam = params.get('path');
			if (pathParam) {
				repoPath = pathParam;
				loadRepo(pathParam).finally(() => {
					startupComplete = true;
				});
			} else {
				getStartupInfo().then((info) => {
					debugOverlayEnabled.set(info.debug_overlay_enabled);
					logPath.set(info.log_path);
					if (info.paths.length > 0) {
						repoPath = info.paths[0];
						loadRepo(info.paths[0]).finally(() => {
							startupComplete = true;
						});
					} else {
						startupComplete = true;
					}
				});
			}
		}

		initPreferences().then(doStartup, doStartup);

		function onResize() {
			viewportHeight = window.innerHeight;
		}
		window.addEventListener('resize', onResize);
		window.addEventListener('keydown', handleKeydown);

		let fpsRafId = 0;
		function fpsLoop() {
			tickFps();
			fpsRafId = requestAnimationFrame(fpsLoop);
		}
		fpsRafId = requestAnimationFrame(fpsLoop);

		startMemoryTracking();

		return () => {
			window.removeEventListener('resize', onResize);
			window.removeEventListener('keydown', handleKeydown);
			cancelAnimationFrame(fpsRafId);
			stopMemoryTracking();
		};
	});

	let loadError = $state<string | null>(null);

	$effect(() => {
		void $locale;
		void recentRepos;
		registerCommands();
	});

	function closeRepo() {
		repoPath = '';
		repoInfo.set(null);
		selectedOid.set(null);
		error.set(null);
		loadError = null;
		comparisonOid.set(null);
		commits = [];
		graphLayout = null;
		allRefs = [];
		workingChangesDiff = null;
		commitDetails = null;
		selectedBranch = null;
		selectedRemote = null;
		selectedTag = null;
		focusBranchOid = null;
		getRecentRepositories().then((r) => (recentRepos = r));
	}

	async function loadRepo(path: string) {
		operationState.set('LoadingRepo');
		error.set(null);
		loadError = null;
		selectedOid.set(null);
		comparisonOid.set(null);
		focusBranchOid = null;
		selectedBranch = null;
		selectedRemote = null;
		selectedTag = null;
		commitDetails = null;
		detailsLoading = false;
		try {
			const data = await getInitialData(path, {
				orientation: $graphOrientation,
				color_mode: $graphColorMode,
				palette: $graphPalette
			});
			const repoRoot = data.repo_info.path;
			repoPath = repoRoot;
			repoInfo.set(data.repo_info);
			repoLoaded = true;
			if (data.repo_info.head_commit) {
				onSelectCommit(data.repo_info.head_commit);
			}
			commits = data.commits;
			graphLayout = data.graph_layout;
			allRefs = data.refs;
			workingChangesDiff = data.working_changes;
			saveRecentRepository(repoRoot).catch(() => {});
			getRecentRepositories().then((r) => (recentRepos = r));
			updateLoadPhaseTimings([
				{ phase: 'load_commits', durationMs: data.timing.load_commits_ms },
				{ phase: 'graph_calc', durationMs: data.timing.graph_calc_ms },
				{ phase: 'refs', durationMs: data.timing.refs_ms },
				{ phase: 'working_changes', durationMs: data.timing.working_changes_ms },
				{ phase: 'total', durationMs: data.timing.total_ms }
			]);
			for (const w of data.warnings) {
				showToast(w, 'warning');
			}
		} catch (e: unknown) {
			const code = e instanceof Error ? e.message : String(e);
			if (code === 'not_a_git_repository') {
				error.set(translate('page.not_a_git_repo', { path }));
			} else if (code === 'open_failed') {
				error.set(translate('page.open_failed', { path }));
			} else {
				loadError = code;
				showToast(translate('page.load_failed'), 'error');
			}
		}

		if (!loadError && commitCount > 0) {
			showToast(translate('page.count_commits', { count: commitCount }), 'info');
		}
		operationState.set('Idle');
	}

	async function browseForRepo() {
		const { open } = await import('@tauri-apps/plugin-dialog');
		const title = translate('page.select_repo_title');
		const selected = await open({ directory: true, multiple: false, title });
		if (selected) {
			repoPath = selected;
			loadRepo(selected);
		}
	}

	async function browseForRepoNewWindow() {
		const { open } = await import('@tauri-apps/plugin-dialog');
		const title = translate('page.select_repo_title');
		const selected = await open({ directory: true, multiple: false, title });
		if (selected) {
			openInNewWindow(selected);
		}
	}

	async function reloadLayout() {
		if (!repoPath) return;
		operationState.set('ApplyingFilter');
		const gen = ++layoutGeneration;
		try {
			const result = await getGraphLayout(repoPath, {
				orientation: $graphOrientation,
				color_mode: $graphColorMode,
				palette: $graphPalette,
				focus_branch_oid: focusBranchOid
			});
			if (gen !== layoutGeneration) return;
			graphLayout = result;
		} catch (e) {
			console.error('Failed to reload graph layout:', e);
		} finally {
			if ($operationState === 'ApplyingFilter') operationState.set('Idle');
		}
	}

	let repoLoaded = $state(false);

	$effect(() => {
		if (startupComplete && !$repoInfo && !autoDialogShown) {
			autoDialogShown = true;
			getRecentRepositories().then((r) => {
				recentRepos = r;
				if (r.length === 0) {
					browseForRepo();
				}
			});
		}
	});

	async function manualRefresh() {
		if (!repoPath) return;
		refreshSignal++;
		loadRepo(repoPath);
	}

	function makeVirtualCommit(oid: string, summary: string, _fileCount: number): CommitInfo {
		return {
			oid,
			short_oid: '',
			message: summary,
			summary,
			author: { name: '', email: '' },
			committer: { name: '', email: '' },
			author_time: '',
			commit_time: '',
			parent_oids: [],
			refs: []
		};
	}

	let allCommits = $derived.by(() => {
		if (!graphLayout || graphLayout.stash_commits.length === 0) return commits;
		return [...commits, ...graphLayout.stash_commits];
	});

	let displayCommits = $derived.by(() => {
		void $locale;
		if (!workingChangesDiff) return allCommits;
		const hasStaged = workingChangesDiff.staged.length > 0;
		const hasUnstaged = workingChangesDiff.unstaged.length > 0;
		if (!hasStaged && !hasUnstaged) return allCommits;
		const virtuals: CommitInfo[] = [];
		if (hasUnstaged)
			virtuals.push(
				makeVirtualCommit(
					UNSTAGED_OID,
					translate('page.unstaged'),
					workingChangesDiff.unstaged.length
				)
			);
		if (hasStaged)
			virtuals.push(
				makeVirtualCommit(STAGED_OID, translate('page.staged'), workingChangesDiff.staged.length)
			);
		return [...virtuals, ...allCommits];
	});

	let displayLayout = $derived.by(() => {
		if (!graphLayout) return null;
		if (!workingChangesDiff) return graphLayout;
		const hasStaged = workingChangesDiff.staged.length > 0;
		const hasUnstaged = workingChangesDiff.unstaged.length > 0;
		if (!hasStaged && !hasUnstaged) return graphLayout;
		const virtualCount = (hasStaged ? 1 : 0) + (hasUnstaged ? 1 : 0);
		const virtualNodes: import('$lib/bindings/types').NodePosition[] = [];
		if (hasUnstaged) {
			virtualNodes.push({
				oid: UNSTAGED_OID,
				row: 0,
				column: 0,
				is_merge: false,
				color: { r: 255, g: 255, b: 255, a: 255 },
				is_dimmed: false,
				is_highlighted: false,
				is_stash: false
			});
		}
		if (hasStaged) {
			virtualNodes.push({
				oid: STAGED_OID,
				row: hasUnstaged ? 1 : 0,
				column: 0,
				is_merge: false,
				color: { r: 255, g: 255, b: 255, a: 255 },
				is_dimmed: false,
				is_highlighted: false,
				is_stash: false
			});
		}
		const virtualEdges: import('$lib/bindings/types').Edge[] = [];
		const headNode = graphLayout.nodes.length > 0 ? graphLayout.nodes[0] : null;
		if (headNode) {
			const headRow = headNode.row + virtualCount;
			const headCol = headNode.column;
			if (hasUnstaged) {
				virtualEdges.push({
					from_row: 0,
					from_col: 0,
					to_row: headRow,
					to_col: headCol,
					edge_type: 'Straight' as const,
					color: { r: 251, g: 146, b: 60, a: 200 },
					is_dimmed: false,
					edge_style: 'Solid' as const
				});
			}
			if (hasStaged) {
				virtualEdges.push({
					from_row: hasUnstaged ? 1 : 0,
					from_col: 0,
					to_row: headRow,
					to_col: headCol,
					edge_type: 'Straight' as const,
					color: { r: 74, g: 222, b: 128, a: 200 },
					is_dimmed: false,
					edge_style: 'Solid' as const
				});
			}
		}
		return {
			...graphLayout,
			nodes: [
				...virtualNodes,
				...graphLayout.nodes.map((n) => ({ ...n, row: n.row + virtualCount }))
			],
			edges: [
				...virtualEdges,
				...graphLayout.edges.map((e) => ({
					...e,
					from_row: e.from_row + virtualCount,
					to_row: e.to_row + virtualCount
				}))
			],
			stash_markers: graphLayout.stash_markers.map((s) => ({ ...s, row: s.row + virtualCount })),
			total_rows: graphLayout.total_rows + virtualCount
		};
	});

	let effectiveCommits = $derived.by(() => {
		let result = displayCommits;

		if ($searchShowMode === 'hide-nonhits' && $matchingOids.size > 0) {
			result = result.filter((c) => VIRTUAL_OIDS.has(c.oid) || $matchingOids.has(c.oid));
		}

		const sb = $sortBy;
		const sa = $sortAsc;
		if (sb !== 'date' || sa) {
			const sorted = [...result];
			const direction = sa ? 1 : -1;
			if (sb === 'date') {
				sorted.sort((a, b) => direction * a.author_time.localeCompare(b.author_time));
			} else if (sb === 'author') {
				sorted.sort((a, b) => direction * a.author.name.localeCompare(b.author.name));
			} else if (sb === 'sha') {
				sorted.sort((a, b) => direction * a.oid.localeCompare(b.oid));
			}
			return sorted;
		}

		return result;
	});

	let hideMergeLayout = $derived(computeHideMergeLayout(displayLayout, $graphHideMerges));

	let effectiveLayout = $derived.by(() => {
		if ($searchShowMode === 'hide-nonhits' && $matchingOids.size > 0) return null;
		if ($sortBy !== 'date' || $sortAsc) return null;
		return hideMergeLayout;
	});

	let displayedCommits = $derived(
		$graphHideMerges ? effectiveCommits.filter((c) => c.parent_oids.length <= 1) : effectiveCommits
	);

	let commitCount = $derived(displayedCommits.length);

	$effect(() => {
		void $graphColorMode;
		void $graphOrientation;
		void $graphPalette;
		untrack(() => {
			if (get(repoInfo)) {
				reloadLayout();
			}
		});
	});

	$effect(() => {
		if (graphLayout) {
			updateDebugGraphStats(
				graphLayout.nodes.length,
				graphLayout.edges.length,
				graphLayout.stash_markers.length,
				graphLayout.total_columns
			);
		}
	});

	$effect(() => {
		if (typeof document === 'undefined') return;
		const root = document.documentElement;
		if ($theme === 'light') {
			root.classList.remove('dark');
			root.classList.add('light');
		} else {
			root.classList.add('dark');
			root.classList.remove('light');
		}
		if ($highContrast) {
			root.classList.add('high-contrast');
		} else {
			root.classList.remove('high-contrast');
		}
		root.style.fontSize = `${$fontSize}px`;
	});

	async function onSelectCommit(oid: string, ctrlKey = false) {
		if (!ctrlKey && oid === $selectedOid) return;
		selectedBranch = null;
		selectedRemote = null;
		selectedTag = null;
		if (ctrlKey && $selectedOid && $selectedOid !== oid) {
			comparisonOid.set(oid);
			return;
		}
		comparisonOid.set(null);
		selectedOid.set(oid);
		commitDetails = null;
		detailsLoading = true;
		operationState.set('LoadingDetails');

		if (VIRTUAL_OIDS.has(oid)) {
			try {
				workingChangesDiff = await getWorkingChanges(repoPath);
			} catch {
				workingChangesDiff = null;
			}
			if (oid !== $selectedOid) {
				detailsLoading = false;
				operationState.set('Idle');
				return;
			}
			const files: FileChange[] =
				oid === STAGED_OID
					? (workingChangesDiff?.staged ?? [])
					: (workingChangesDiff?.unstaged ?? []);
			if (files.length === 0) {
				detailsLoading = false;
				operationState.set('Idle');
				if ($selectedOid === oid) selectedOid.set(null);
				return;
			}
			const label = oid === STAGED_OID ? translate('page.staged') : translate('page.unstaged');
			commitDetails = {
				info: {
					oid,
					short_oid: '',
					message: label,
					summary: label,
					author: { name: '', email: '' },
					committer: { name: '', email: '' },
					author_time: '',
					commit_time: '',
					parent_oids: [],
					refs: []
				},
				tree_oid: '',
				signature: null,
				changed_files: files,
				body: null
			};
			detailsLoading = false;
			if ($operationState === 'LoadingDetails') operationState.set('Idle');
			return;
		}

		try {
			commitDetails = await getCommitDetails(repoPath, oid);
			if (oid !== $selectedOid) {
				commitDetails = null;
				return;
			}
		} catch {
			commitDetails = null;
		} finally {
			detailsLoading = false;
			if ($operationState === 'LoadingDetails') operationState.set('Idle');
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if ((e.key === 'O' || e.key === 'o') && (e.ctrlKey || e.metaKey) && e.shiftKey) {
			e.preventDefault();
			browseForRepoNewWindow();
			return;
		}
		if (e.key === 'o' && (e.ctrlKey || e.metaKey) && !e.shiftKey) {
			e.preventDefault();
			browseForRepo();
			return;
		}
		if (e.key === 'F12' || (e.key === 'D' && e.ctrlKey && e.shiftKey)) {
			e.preventDefault();
			if ($debugOverlayEnabled) {
				toggleDebug();
			}
			return;
		}
		if ((e.key === 'p' && e.ctrlKey) || (e.key === 'p' && e.metaKey)) {
			e.preventDefault();
			showCommandPalette = true;
			return;
		}
		if (e.key === 'm' && e.ctrlKey) {
			e.preventDefault();
			isFullscreen = !isFullscreen;
			return;
		}
		if (e.key === 'w' && (e.ctrlKey || e.metaKey) && $repoInfo) {
			e.preventDefault();
			closeRepo();
			return;
		}
		if (e.key === 'q' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			quitApp();
			return;
		}
		if ((e.key === 'F1' || (e.key === '/' && (e.ctrlKey || e.metaKey))) && !showPreferences) {
			e.preventDefault();
			showShortcutHelp = true;
			return;
		}
		if (e.key === 'Escape') {
			if (showCommandPalette || contextMenu || showShortcutHelp || showPreferences || showInfo)
				return;
			if (isFullscreen) {
				isFullscreen = false;
				return;
			}
			comparisonOid.set(null);
			return;
		}
		if (e.key === 'b' && e.altKey && !e.shiftKey && branchNames.length > 0) {
			e.preventDefault();
			const currentIdx = selectedBranch ? branchNames.indexOf(selectedBranch) : -1;
			const nextIdx = currentIdx < branchNames.length - 1 ? currentIdx + 1 : 0;
			handleBranchSelect(branchNames[nextIdx]);
			return;
		}
		if ((e.key === 'B' || e.key === 'b') && e.altKey && e.shiftKey && branchNames.length > 0) {
			e.preventDefault();
			const currentIdx = selectedBranch ? branchNames.indexOf(selectedBranch) : branchNames.length;
			const prevIdx = currentIdx > 0 ? currentIdx - 1 : branchNames.length - 1;
			handleBranchSelect(branchNames[prevIdx]);
			return;
		}
		if ((e.key === 'r' || e.key === 'R') && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			manualRefresh();
			return;
		}
		if (!$selectedOid || !commits.length) return;

		const currentCommit = commits.find((c) => c.oid === $selectedOid);
		if (!currentCommit) return;

		if (e.key === 'n' && e.altKey) {
			e.preventDefault();
			navigateAuthor(currentCommit, 1);
		} else if (e.key === 'p' && e.altKey) {
			e.preventDefault();
			navigateAuthor(currentCommit, -1);
		}
	}

	function navigateAuthor(current: CommitInfo, direction: 1 | -1) {
		const authorKey = current.author.name + ' <' + current.author.email + '>';
		const idx = commits.findIndex((c) => c.oid === current.oid);
		if (idx < 0) return;

		for (let i = idx + direction; i >= 0 && i < commits.length; i += direction) {
			const c = commits[i];
			const key = c.author.name + ' <' + c.author.email + '>';
			if (key === authorKey) {
				onSelectCommit(c.oid);
				return;
			}
		}
	}

	function persistDetailPanelHeight() {
		updateLayout({ detailPanelHeight });
	}

	function registerCommands() {
		registerCommand({
			id: 'open-repo',
			label: translate('page.cmd_open_repo'),
			shortcut: 'Ctrl+O',
			category: 'File',
			action: browseForRepo
		});
		registerCommand({
			id: 'open-repo-new-window',
			label: translate('page.cmd_open_repo_new_window'),
			shortcut: 'Ctrl+Shift+O',
			category: 'File',
			action: browseForRepoNewWindow
		});
		registerCommand({
			id: 'close-repo',
			label: translate('page.cmd_close_repo'),
			shortcut: 'Ctrl+W',
			category: 'File',
			action: closeRepo
		});
		registerCommand({
			id: 'quit',
			label: translate('page.cmd_quit'),
			shortcut: 'Ctrl+Q',
			category: 'File',
			action: quitApp
		});
		registerCommand({
			id: 'refresh',
			label: translate('page.cmd_refresh'),
			shortcut: 'Ctrl+R',
			category: 'File',
			action: manualRefresh
		});

		unregisterCommandsByPrefix('recent-repo-');
		recentRepos.forEach((repo, i) => {
			registerCommand({
				id: `recent-repo-${i}`,
				label: repo.name,
				category: 'Recent',
				action: () => {
					repoPath = repo.path;
					loadRepo(repo.path);
				}
			});
		});

		registerCommand({
			id: 'toggle-merges',
			label: translate('page.cmd_toggle_merges'),
			shortcut: undefined,
			category: 'Graph',
			action: () => graphHideMerges.update((v) => !v)
		});
		registerCommand({
			id: 'toggle-color-mode',
			label: translate('page.cmd_color_author'),
			shortcut: undefined,
			category: 'Graph',
			action: () => graphColorMode.update((v) => (v === 'by-branch' ? 'by-author' : 'by-branch'))
		});
		registerCommand({
			id: 'toggle-orientation',
			label: translate('page.cmd_orientation'),
			shortcut: undefined,
			category: 'Graph',
			action: () =>
				graphOrientation.update((v) => (v === 'top-to-bottom' ? 'bottom-to-top' : 'top-to-bottom'))
		});
		registerCommand({
			id: 'toggle-debug',
			label: translate('page.cmd_debug'),
			shortcut: 'F12',
			category: 'Debug',
			action: () => {
				if ($debugOverlayEnabled) toggleDebug();
			}
		});
		registerCommand({
			id: 'palette-default',
			label: translate('page.cmd_palette_default'),
			category: 'Palette',
			action: () => graphPalette.set('default')
		});
		registerCommand({
			id: 'palette-deuteranopia',
			label: translate('page.cmd_palette_deuteranopia'),
			category: 'Palette',
			action: () => graphPalette.set('deuteranopia')
		});
		registerCommand({
			id: 'palette-protanopia',
			label: translate('page.cmd_palette_protanopia'),
			category: 'Palette',
			action: () => graphPalette.set('protanopia')
		});
		registerCommand({
			id: 'palette-tritanopia',
			label: translate('page.cmd_palette_tritanopia'),
			category: 'Palette',
			action: () => graphPalette.set('tritanopia')
		});
		registerCommand({
			id: 'toggle-fullscreen',
			label: translate('page.cmd_fullscreen'),
			shortcut: 'Ctrl+M',
			category: 'View',
			action: () => {
				isFullscreen = !isFullscreen;
			}
		});
		registerCommand({
			id: 'show-shortcuts',
			label: translate('page.cmd_shortcuts'),
			shortcut: 'F1',
			category: 'Help',
			action: () => {
				showShortcutHelp = true;
			}
		});
		registerCommand({
			id: 'branch-next',
			label: translate('page.cmd_branch_next'),
			shortcut: 'Alt+B',
			category: 'Branch',
			action: () => {
				if (branchNames.length === 0) return;
				const currentIdx = selectedBranch ? branchNames.indexOf(selectedBranch) : -1;
				const nextIdx = currentIdx < branchNames.length - 1 ? currentIdx + 1 : 0;
				handleBranchSelect(branchNames[nextIdx]);
			}
		});
		registerCommand({
			id: 'branch-prev',
			label: translate('page.cmd_branch_prev'),
			shortcut: 'Alt+Shift+B',
			category: 'Branch',
			action: () => {
				if (branchNames.length === 0) return;
				const currentIdx = selectedBranch
					? branchNames.indexOf(selectedBranch)
					: branchNames.length;
				const prevIdx = currentIdx > 0 ? currentIdx - 1 : branchNames.length - 1;
				handleBranchSelect(branchNames[prevIdx]);
			}
		});
		registerCommand({
			id: 'show-info',
			label: translate('toolbar.info'),
			category: 'Help',
			action: () => {
				showInfo = true;
			}
		});
	}

	function handleCommitContextMenu(e: MouseEvent, oid: string) {
		e.preventDefault();
		const commit = commits.find((c) => c.oid === oid);
		const items: ContextMenuItem[] = [
			{ label: translate('page.ctx_copy_sha'), action: () => navigator.clipboard.writeText(oid) },
			{
				label: translate('page.ctx_copy_short_sha'),
				action: () => navigator.clipboard.writeText(oid.substring(0, 7))
			}
		];
		if (commit) {
			items.push({
				label: translate('page.ctx_copy_message'),
				action: () => navigator.clipboard.writeText(commit.message)
			});
			items.push({ separator: true });
			if ($selectedOid && $selectedOid !== oid) {
				items.push({ label: translate('page.ctx_compare'), action: () => comparisonOid.set(oid) });
			}
		}
		contextMenu = { x: e.clientX, y: e.clientY, items };
	}

	function handleBranchContextMenu(e: MouseEvent, name: string) {
		const items: ContextMenuItem[] = [
			{
				label: translate('page.ctx_copy_branch'),
				action: () => navigator.clipboard.writeText(name)
			}
		];
		contextMenu = { x: e.clientX, y: e.clientY, items };
	}

	function handleBranchSelect(name: string) {
		const ref = allRefs.find((r) => 'Branch' in r && r.Branch?.name === name);
		if (ref && 'Branch' in ref && ref.Branch) {
			if (selectedBranch === name) {
				selectedBranch = null;
				focusBranchOid = null;
				reloadLayout();
				return;
			}
			onSelectCommit(ref.Branch.oid);
			selectedBranch = name;
			selectedRemote = null;
			selectedTag = null;
			focusBranchOid = ref.Branch.oid;
			reloadLayout();
		}
	}

	function handleTagSelect(name: string) {
		const ref = allRefs.find((r) => 'Tag' in r && r.Tag?.name === name);
		if (ref && 'Tag' in ref && ref.Tag) {
			if (selectedTag === name) {
				selectedTag = null;
				focusBranchOid = null;
				reloadLayout();
				return;
			}
			selectedBranch = null;
			selectedRemote = null;
			onSelectCommit(ref.Tag.oid);
			selectedTag = name;
			focusBranchOid = ref.Tag.oid;
			reloadLayout();
		}
	}

	function handleRemoteSelect(remote: string, name: string) {
		const key = `${remote}/${name}`;
		const ref = allRefs.find(
			(r) => 'Remote' in r && r.Remote?.remote === remote && r.Remote?.name === name
		);
		if (ref && 'Remote' in ref && ref.Remote) {
			if (selectedRemote === key) {
				selectedRemote = null;
				focusBranchOid = null;
				reloadLayout();
				return;
			}
			selectedBranch = null;
			selectedTag = null;
			onSelectCommit(ref.Remote.oid);
			selectedRemote = key;
			focusBranchOid = ref.Remote.oid;
			reloadLayout();
		}
	}

	function handleRemoteContextMenu(e: MouseEvent, remote: string, name: string) {
		const items: ContextMenuItem[] = [
			{
				label: translate('page.ctx_copy_remote'),
				action: () => navigator.clipboard.writeText(`${remote}/${name}`)
			}
		];
		contextMenu = { x: e.clientX, y: e.clientY, items };
	}

	function handleDragOver(e: DragEvent) {
		if (e.dataTransfer?.types.includes('Files')) {
			e.preventDefault();
			e.dataTransfer.dropEffect = 'move';
			isDragging = true;
		}
	}

	function handleDragLeave() {
		isDragging = false;
	}

	function formatAbsoluteDate(iso: string): string {
		if (!iso) return '';
		try {
			const d = new Date(iso);
			if (isNaN(d.getTime())) return iso;
			return d.toLocaleDateString(undefined, {
				year: 'numeric',
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return iso;
		}
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
		const files = e.dataTransfer?.files;
		if (!files || files.length === 0) return;
		const file = files[0] as File & { path?: string };
		if (file.path) {
			repoPath = file.path;
			loadRepo(file.path);
		}
	}
</script>

<div
	class="flex h-screen flex-col bg-gray-900 text-gray-100 {isDragging
		? 'ring-2 ring-blue-500 ring-inset'
		: ''}"
	role="application"
	ondragover={handleDragOver}
	ondragleave={handleDragLeave}
	ondrop={handleDrop}
>
	{#if !$repoInfo || !repoLoaded}
		<div class="flex flex-1 items-center justify-center" role="main">
			{#if $operationState === 'LoadingRepo'}
				<div class="flex flex-col items-center gap-3">
					<div
						class="h-8 w-8 animate-spin rounded-full border-2 border-gray-600 border-t-blue-500"
					></div>
					<span class="text-sm text-gray-500">{$t('page.loading_repo')}</span>
				</div>
			{:else}
				<div class="w-full max-w-md space-y-6 p-8">
					<div class="space-y-2 text-center">
						<h1 class="text-2xl font-bold">{$t('page.title')}</h1>
						<p class="text-gray-400">{$t('page.subtitle')}</p>
					</div>
					<div class="flex justify-center">
						<button
							class="rounded bg-blue-600 px-6 py-3 text-sm hover:bg-blue-700"
							onclick={browseForRepo}
							aria-label={$t('page.browse_repo')}
						>
							{$t('page.browse_repo')}
						</button>
					</div>
					{#if $error}
						<p class="text-sm text-red-400 text-center" role="alert">{$error}</p>
					{/if}
					{#if recentRepos.length > 0}
						<div class="space-y-2">
							<h2 class="text-xs font-semibold uppercase tracking-wider text-gray-500">
								{$t('page.recent_title')}
							</h2>
							<ul class="space-y-1">
								{#each recentRepos as repo (repo.path)}
									<li>
										<button
											class="w-full rounded px-3 py-2 text-left text-sm text-gray-300 transition-colors hover:bg-gray-800"
											onclick={() => {
												repoPath = repo.path;
												loadRepo(repo.path);
											}}
											aria-label={$t('page.recent_aria', { name: repo.name })}
										>
											<span class="font-medium">{repo.name}</span>
											<span class="ml-2 font-mono text-xs text-gray-600">{repo.path}</span>
											<span class="ml-auto text-xs text-gray-600"
												>{formatAbsoluteDate(repo.last_opened)}</span
											>
										</button>
									</li>
								{/each}
							</ul>
						</div>
					{:else}
						<p class="text-center text-sm text-gray-500">{$t('page.open_first')}</p>
					{/if}
					<div class="flex justify-center gap-1">
						<button
							class="rounded p-2 text-gray-400 hover:bg-gray-800 hover:text-white transition-colors"
							onclick={() => (showPreferences = true)}
							aria-label={$t('page.settings_aria')}
						>
							<svg
								class="h-5 w-5"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
								aria-hidden="true"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
								/>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
								/>
							</svg>
						</button>
						<button
							class="rounded p-2 text-gray-400 hover:bg-gray-800 hover:text-white transition-colors"
							onclick={() => (showInfo = true)}
							aria-label={$t('toolbar.info_aria')}
						>
							<svg
								class="h-5 w-5"
								fill="none"
								viewBox="0 0 24 24"
								stroke="currentColor"
								aria-hidden="true"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									stroke-width="2"
									d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
								/>
							</svg>
						</button>
					</div>
				</div>
			{/if}
		</div>
	{:else}
		{#if !isFullscreen}
			<header
				class="relative flex items-center gap-3 overflow-visible border-b border-gray-800 px-4 py-2"
			>
				<button
					class="rounded bg-gray-700 px-3 py-1.5 text-xs text-gray-200 hover:bg-gray-600 shrink-0"
					onclick={browseForRepo}
					aria-label={$t('page.browse_repo')}
				>
					{$t('page.browse_repo')}
				</button>
				{#each recentRepos as repo (repo.path)}
					<button
						class="rounded bg-gray-700/50 px-2 py-0.5 text-xs text-gray-300 hover:bg-gray-600 hover:text-gray-100 shrink-0"
						onclick={() => {
							repoPath = repo.path;
							loadRepo(repo.path);
						}}
						title={repo.path}
					>
						{repo.name}
					</button>
				{/each}
				<Toolbar
					onrefresh={manualRefresh}
					onopensettings={() => (showPreferences = true)}
					onopeninfo={() => (showInfo = true)}
				/>
				{#if graphLayout}
					<AuthorLegend layout={graphLayout} />
				{/if}
				<div class="ml-auto flex items-center gap-3">
					<SearchBar {repoPath} />
					{#if loadError}
						<span class="flex items-center gap-2 text-xs text-amber-400">
							{$t('page.loading_incomplete')}
							<button
								class="rounded bg-amber-700/50 px-2 py-0.5 text-xs hover:bg-amber-700"
								onclick={() => loadRepo(repoPath)}
								aria-label={$t('page.retry')}
							>
								{$t('page.retry')}
							</button>
						</span>
					{/if}
				</div>
			</header>
		{/if}
		<div class="flex flex-1 overflow-hidden">
			{#if !isFullscreen}
				<Sidebar gotoTab={sidebarGotoTab} width={sidebarWidth}>
					{#snippet refs()}
						<RefList
							refs={allRefs}
							{selectedBranch}
							{selectedRemote}
							{selectedTag}
							onbranchselect={handleBranchSelect}
							onbranchcontextmenu={handleBranchContextMenu}
							ontagselect={handleTagSelect}
							onremoteselect={handleRemoteSelect}
							onremotecontextmenu={handleRemoteContextMenu}
						/>
					{/snippet}
					{#snippet stash()}
						<StashList
							{repoPath}
							{refreshSignal}
							onstashselect={(stash) => onSelectCommit(stash.oid)}
						/>
					{/snippet}
					{#snippet reflog()}
						<ReflogPanel {repoPath} refs={allRefs} onentryselect={(oid) => onSelectCommit(oid)} />
					{/snippet}
					{#snippet history()}
						{#if historyFilePath}
							<FileHistoryPanel
								{repoPath}
								filePath={historyFilePath}
								revision={historyRevision}
								onenterselect={(oid: string) => onSelectCommit(oid)}
							/>
						{:else}
							<div class="text-gray-500 italic">{$t('page.no_file_selected')}</div>
						{/if}
					{/snippet}
				</Sidebar>
			{/if}
			<div
				class="flex-1 min-h-0 overflow-hidden flex flex-col"
				role="main"
				aria-label={$t('commit_list.aria')}
			>
				<div class="flex-1 min-h-0 overflow-hidden">
					{#if displayedCommits.length > 0}
						{#key $repoInfo?.path ?? ''}
							<CommitList
								commits={displayedCommits}
								layout={effectiveLayout}
								selectedOid={$selectedOid}
								comparisonOid={$comparisonOid}
								matchingOids={$matchingOids.size > 0 ? $matchingOids : undefined}
								onSelect={(oid: string, ctrlKey: boolean) => onSelectCommit(oid, ctrlKey)}
								onContextMenu={handleCommitContextMenu}
								graphWidth={savedLayout?.graphWidth ?? 200}
								{rowHeight}
							/>
						{/key}
					{:else if $operationState === 'LoadingRepo'}
						<div class="flex h-full items-center justify-center" role="status" aria-live="polite">
							<div class="flex flex-col items-center gap-3">
								<div
									class="h-8 w-8 animate-spin rounded-full border-2 border-gray-600 border-t-blue-500"
								></div>
								<span class="text-sm text-gray-500">{$t('page.loading_repo')}</span>
							</div>
						</div>
					{/if}
				</div>

				{#if $selectedOid}
					<ResizeHandle
						bind:panelHeight={detailPanelHeight}
						maxHeight={Math.floor(viewportHeight * 0.9)}
						onDragEnd={persistDetailPanelHeight}
					/>
					<div
						class="overflow-hidden bg-gray-900 border-t border-gray-700"
						style="height: {detailPanelHeight}px;"
						role="region"
						aria-label={$t('commit_detail.diff_viewer')}
					>
						{#if $comparisonOid}
							<ComparisonPanel {repoPath} fromOid={$selectedOid} toOid={$comparisonOid} />
						{:else if detailsLoading}
							<div
								class="flex items-center justify-center h-full text-sm text-gray-500"
								role="status"
								aria-live="polite"
							>
								{$t('page.loading_details')}
							</div>
						{:else if commitDetails}
							<CommitDetailPanel
								details={commitDetails}
								{repoPath}
								onhistoryfile={(p: string) => {
									historyFilePath = p;
									historyRevision++;
									sidebarGotoTab = 'history';
								}}
								oncommitselect={(oid) => onSelectCommit(oid)}
							/>
						{:else}
							<div
								class="flex items-center justify-center h-full text-sm text-gray-500"
								role="alert"
							>
								{$t('page.failed_details')}
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
	{#if $repoInfo}
		<footer
			class="flex items-center gap-3 border-t border-gray-800 px-4 py-1 text-xs text-gray-500 shrink-0"
			role="status"
			aria-label={$t('page.statusbar_aria')}
		>
			<span class="font-mono text-gray-400">{$repoInfo.path}</span>
			{#if selectedBranch}
				<span class="rounded bg-blue-700/50 px-2 py-0.5 text-xs text-blue-300">
					{selectedBranch}
				</span>
			{:else if selectedRemote}
				<span class="rounded bg-purple-700/50 px-2 py-0.5 text-xs text-purple-300">
					{selectedRemote}
				</span>
			{:else if selectedTag}
				<span class="rounded bg-yellow-700/50 px-2 py-0.5 text-xs text-yellow-300">
					{selectedTag}
				</span>
			{:else if $repoInfo.head_branch}
				<span class="rounded bg-green-700/50 px-2 py-0.5 text-xs text-green-300">
					{$repoInfo.head_branch}
				</span>
			{:else if detachedHeadSha}
				<span class="rounded bg-red-700/50 px-2 py-0.5 text-xs text-red-300">
					{$t('page.detached_head', { sha: detachedHeadSha })}
				</span>
			{/if}
			{#if $repoInfo.is_bare}
				<span class="rounded bg-gray-700/50 px-2 py-0.5 text-xs text-gray-400">
					{$t('page.bare_repo')}
				</span>
			{/if}
			<span>{$t('page.commits_label', { count: commitCount })}</span>
			{#if uncommittedCount > 0}
				<span class="rounded bg-amber-700/50 px-2 py-0.5 text-amber-300">
					{$t(
						uncommittedCount === 1 ? 'page.uncommitted_changes' : 'page.uncommitted_changes_plural',
						{ count: uncommittedCount }
					)}
				</span>
			{/if}
			{#if $graphHideMerges}
				<span class="rounded bg-yellow-700/50 px-2 py-0.5 text-yellow-300">
					{$t('page.merges_hidden')}
				</span>
			{/if}
			{#if $operationState !== 'Idle'}
				<span class="text-blue-400">
					{$t('page.' + $operationState.toLowerCase()) ?? $operationState}
				</span>
			{/if}
		</footer>
	{/if}
	<ToastContainer />
	<DebugOverlay />
	{#if showPreferences}
		<PreferencesModal onclose={() => (showPreferences = false)} />
	{/if}
	{#if showShortcutHelp}
		<ShortcutHelp onclose={() => (showShortcutHelp = false)} />
	{/if}
	{#if showInfo}
		<InfoDialog onclose={() => (showInfo = false)} />
	{/if}
	{#if showCommandPalette}
		<CommandPalette onclose={() => (showCommandPalette = false)} />
	{/if}
	{#if contextMenu}
		<ContextMenu
			x={contextMenu.x}
			y={contextMenu.y}
			items={contextMenu.items}
			onclose={() => (contextMenu = null)}
		/>
	{/if}
	<div class="sr-only" aria-live="polite" role="status" id="a11y-announcer"></div>
</div>
