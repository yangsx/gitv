<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { untrack } from 'svelte';
	import { get } from 'svelte/store';
	import {
		getGraphLayout,
		getCommitDetails,
		getCommitFileCounts,
		getCommitsBatch,
		getDiff,
		getInitialData,
		getWorkingChanges,
		getStartupInfo,
		getRecentRepositories,
		saveRecentRepository,
		openInNewWindow,
		setWindowTitle,
		quitApp,
		cancelPatchSearch
	} from '$lib/bindings/commands';
	import type {
		CommitInfo,
		GraphLayout,
		CommitDetails,
		DiffSummary,
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
		arrowGapThreshold,
		operationState,
		sortBy,
		sortAsc,
		searchShowMode,
		patchSearchActive,
		patchSearchProgress,
		patchSearchId,
		searchQuery,
		searchResults
	} from '$lib/stores/repository';
	import { STAGED_OID, VIRTUAL_OIDS } from '$lib/constants';
	import CommitList from '$lib/components/CommitList.svelte';
	import SearchBar from '$lib/components/SearchBar.svelte';
	import CommitDetailPanel from '$lib/components/CommitDetailPanel.svelte';
	import ResizeHandle from '$lib/components/ResizeHandle.svelte';
	import Sidebar from '$lib/components/Sidebar/Sidebar.svelte';
	import RefList from '$lib/components/Sidebar/RefList.svelte';
	import StashList from '$lib/components/Sidebar/StashList.svelte';
	import ReflogPanel from '$lib/components/Sidebar/ReflogPanel.svelte';
	import FileHistoryPanel from '$lib/components/FileHistoryPanel.svelte';
	import Toolbar from '$lib/components/Toolbar.svelte';
	import AuthorLegend from '$lib/components/AuthorLegend.svelte';
	import ToastContainer from '$lib/components/ToastContainer.svelte';
	import { showToast, updateToast, dismissToast } from '$lib/stores/toast';
	import {
		toggleDebug,
		tickFps,
		updateDebugGraphStats,
		updateLoadPhaseTimings,
		logPath,
		startMemoryTracking,
		stopMemoryTracking
	} from '$lib/stores/debug';
	import DebugOverlay from '$lib/components/DebugOverlay.svelte';
	import {
		getClampedLayout,
		updateLayout,
		restoreWindowGeometry,
		saveWindowGeometry
	} from '$lib/stores/layout';
	import { registerCommand, unregisterCommandsByPrefix } from '$lib/stores/commands';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import ContextMenu from '$lib/components/ContextMenu.svelte';
	import type { ContextMenuItem } from '$lib/components/ContextMenu.svelte';
	import PreferencesModal from '$lib/components/PreferencesModal.svelte';
	import InfoDialog from '$lib/components/InfoDialog.svelte';
	import {
		initPreferences,
		resolvedTheme,
		fontSize,
		adjustFontSize,
		resolvedHighContrast
	} from '$lib/stores/preferences';
	import { t, translate, locale } from '$lib/stores/locale';
	import { computeHideMergeLayout } from '$lib/graph/hide-merges';
	import {
		applyVirtualWorkingChanges,
		createVirtualCommitInfos
	} from '$lib/graph/virtual-working-changes';
	import { announce } from '$lib/utils/a11y';

	let repoPath = $state('');
	let startupComplete = $state(false);
	let autoDialogShown = $state(false);
	let recentRepos = $state<RecentRepository[]>([]);
	let commits = $state<CommitInfo[]>([]);
	let totalCommitCount = $state(0);
	let loadingMoreCommits = $state(false);
	const COMMIT_BATCH_SIZE = 1000;
	let graphLayout = $state<GraphLayout | null>(null);
	let layoutGeneration = 0;
	let layoutToastId: number | null = null;
	let commitDetails = $state<CommitDetails | null>(null);
	let detailsLoading = $state(false);
	let comparisonDetails = $state<CommitDetails | null>(null);
	let comparisonLoading = $state(false);
	let savedLayout = typeof window !== 'undefined' ? getClampedLayout() : null;
	let detailPanelHeight = $state(savedLayout?.detailPanelHeight ?? 400);
	let viewportHeight = $state(typeof window !== 'undefined' ? window.innerHeight : 800);
	let allRefs = $state<Ref[]>([]);
	let historyFilePath = $state<string | null>(null);
	let historyRevision = $state(0);
	let sidebarGotoTab = $state<'refs' | 'stash' | 'reflog' | 'history'>('refs');
	let workingChangesDiff = $state<WorkingChangesDiff | null>(null);
	let showCommandPalette = $state(false);
	let compareMode = $state(false);
	let contextMenu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null);
	let isDragging = $state(false);
	let isFullscreen = $state(false);
	let selectedBranch = $state<string | null>(null);
	let selectedRemote = $state<string | null>(null);
	let selectedTag = $state<string | null>(null);
	let showPreferences = $state(false);
	let showInfo = $state(false);

	let savedFocus: HTMLElement | null = null;

	function openModal(modal: 'preferences' | 'info' | 'commandPalette') {
		savedFocus = document.activeElement as HTMLElement | null;
		if (modal === 'preferences') showPreferences = true;
		else if (modal === 'info') showInfo = true;
		else if (modal === 'commandPalette') showCommandPalette = true;
	}

	function restoreFocus() {
		savedFocus?.focus?.();
		savedFocus = null;
	}

	let uncommittedCount = $derived(
		workingChangesDiff ? workingChangesDiff.staged.length + workingChangesDiff.unstaged.length : 0
	);
	let detachedHeadSha = $derived(
		$repoInfo?.head_commit && !$repoInfo?.head_branch ? $repoInfo.head_commit.substring(0, 7) : null
	);
	let patchMatches = $derived(
		$searchResults
			.filter((r) => r.commit_oid === $selectedOid && r.match_type === 'Patch')
			.flatMap((r) => r.patch_matches)
	);

	const ROW_HEIGHT_REM = 1.75;
	let rowHeight = $derived(Math.round(ROW_HEIGHT_REM * $fontSize));

	let branchNames = $derived(
		(() => {
			const branches = allRefs.filter((r) => 'Branch' in r);
			const headNames = new Set(
				branches.filter((r) => r.Branch!.is_head).map((r) => r.Branch!.name)
			);
			return branches
				.map((r) => r.Branch!.name)
				.sort((a, b) => {
					const aIsHead = headNames.has(a);
					const bIsHead = headNames.has(b);
					if (aIsHead !== bIsHead) return aIsHead ? -1 : 1;
					return a.localeCompare(b);
				});
		})()
	);
	let sidebarWidth = $state(savedLayout?.sidebarWidth ?? 220);

	let focusBranchOid = $state<string | null>(null);
	let refreshSignal = $state(0);
	const modKey =
		typeof navigator !== 'undefined' && /Mac|iPhone|iPad|iPod/i.test(navigator.platform)
			? '⌘'
			: 'Ctrl+';

	function copyToClipboard(text: string) {
		navigator.clipboard
			.writeText(text)
			.catch(() => showToast(translate('page.copy_failed'), 'error'));
	}

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
		restoreWindowGeometry();

		function onResize() {
			viewportHeight = window.innerHeight;
		}
		window.addEventListener('resize', onResize);
		window.addEventListener('keydown', handleKeydown);
		window.addEventListener('wheel', handleWheel, { passive: false });

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
			window.removeEventListener('wheel', handleWheel);
			cancelAnimationFrame(fpsRafId);
			stopMemoryTracking();
			saveWindowGeometry();
		};
	});

	let loadError = $state<string | null>(null);

	$effect(() => {
		void $locale;
		void recentRepos;
		registerCommands();
	});

	function resetRepoState() {
		if ($patchSearchId !== null) cancelPatchSearch($patchSearchId).catch(() => {});
		patchSearchActive.set(false);
		patchSearchId.set(null);
		patchSearchProgress.set(null);
		selectedOid.set(null);
		error.set(null);
		loadError = null;
		comparisonOid.set(null);
		compareMode = false;
		commits = [];
		totalCommitCount = 0;
		graphLayout = null;
		allRefs = [];
		workingChangesDiff = null;
		commitDetails = null;
		detailsLoading = false;
		comparisonDetails = null;
		comparisonLoading = false;
		selectedBranch = null;
		selectedRemote = null;
		selectedTag = null;
		focusBranchOid = null;
		historyFilePath = null;
		historyRevision = 0;
		searchQuery.set(null);
		searchResults.set([]);
	}

	function closeRepo() {
		resetRepoState();
		repoPath = '';
		repoInfo.set(null);
		repoLoaded = false;
		getRecentRepositories().then((r) => (recentRepos = r));
	}

	async function loadRepo(path: string) {
		resetRepoState();
		operationState.set('LoadingRepo');
		try {
			const data = await getInitialData(path, {
				orientation: $graphOrientation,
				color_mode: $graphColorMode,
				arrow_gap_threshold: $arrowGapThreshold,
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
			totalCommitCount = data.total_commit_count;
			graphLayout = data.graph_layout;
			allRefs = data.refs;
			workingChangesDiff = data.working_changes;
			await saveRecentRepository(repoRoot).catch(() => {});
			recentRepos = await getRecentRepositories();
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
				announce(translate('page.not_a_git_repo', { path }));
			} else if (code === 'open_failed') {
				error.set(translate('page.open_failed', { path }));
				announce(translate('page.open_failed', { path }));
			} else {
				loadError = code;
				showToast(translate('page.load_failed'), 'error');
				announce(translate('page.load_failed'));
			}
		}

		if (!loadError && commitCount > 0) {
			showToast(translate('page.count_commits', { count: commitCount }), 'info');
			announce(translate('page.count_commits', { count: commitCount }));
		}
		operationState.set('Idle');

		if (!loadError && commitCount > 0) {
			await tick();
			const commitListEl = document.querySelector<HTMLElement>(
				'#commit-list-container [tabindex="0"]'
			);
			commitListEl?.focus();
		}
	}

	async function loadMoreCommits() {
		if (loadingMoreCommits || !repoPath) return;
		if (commits.length >= totalCommitCount) return;
		loadingMoreCommits = true;
		try {
			const batch = await getCommitsBatch(repoPath, commits.length, COMMIT_BATCH_SIZE);
			if (batch.length > 0) {
				commits = [...commits, ...batch];
			}
		} catch {
			// silent — next scroll event will retry
		} finally {
			loadingMoreCommits = false;
		}
	}

	async function browseForRepo() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const title = translate('page.select_repo_title');
			const selected = await open({ directory: true, multiple: false, title });
			if (selected) {
				repoPath = selected;
				loadRepo(selected);
			}
		} catch {
			showToast(translate('page.dialog_failed'), 'error');
		}
	}

	async function browseForRepoNewWindow() {
		try {
			const { open } = await import('@tauri-apps/plugin-dialog');
			const title = translate('page.select_repo_title');
			const selected = await open({ directory: true, multiple: false, title });
			if (selected) {
				await openInNewWindow(selected);
			}
		} catch {
			showToast(translate('page.dialog_failed'), 'error');
		}
	}

	async function reloadLayout() {
		if (!repoPath) return;
		operationState.set('ApplyingFilter');
		const gen = ++layoutGeneration;
		if (layoutToastId !== null) dismissToast(layoutToastId);
		layoutToastId = showToast($t('toast.reload_layout'), 'info');
		try {
			const result = await getGraphLayout(repoPath, {
				orientation: $graphOrientation,
				color_mode: $graphColorMode,
				palette: $graphPalette,
				arrow_gap_threshold: $arrowGapThreshold,
				focus_branch_oid: focusBranchOid
			});
			if (gen !== layoutGeneration) return;
			graphLayout = result;
			if (layoutToastId !== null) {
				updateToast(layoutToastId, $t('toast.reload_layout_done'), 'info');
				layoutToastId = null;
			}
		} catch (e) {
			console.error('Failed to reload graph layout:', e);
			if (layoutToastId !== null) {
				dismissToast(layoutToastId);
				layoutToastId = null;
			}
			showToast($t('toast.reload_layout_failed'), 'error');
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
		operationState.set('LoadingRepo');
		loadRepo(repoPath);
	}

	let allCommits = $derived.by(() => {
		if (!graphLayout || graphLayout.stash_commits.length === 0) return commits;
		return [...commits, ...graphLayout.stash_commits];
	});

	let displayCommits = $derived.by(() => {
		void $locale;
		return [...createVirtualCommitInfos(workingChangesDiff, translate), ...allCommits];
	});

	let displayLayout = $derived(
		applyVirtualWorkingChanges(graphLayout, workingChangesDiff, $repoInfo?.head_commit)
	);

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

	let displayedCommits = $derived.by(() => {
		if (!$graphHideMerges) return effectiveCommits;
		if (!hideMergeLayout) return effectiveCommits.filter((c) => c.parent_oids.length <= 1);
		const visibleOids = new Set(hideMergeLayout.nodes.map((n) => n.oid));
		return effectiveCommits.filter((c) => visibleOids.has(c.oid));
	});

	let commitCount = $derived(displayedCommits.length);

	$effect(() => {
		void $graphColorMode;
		void $graphOrientation;
		void $graphPalette;
		void $arrowGapThreshold;
		untrack(() => {
			if (get(repoInfo)) {
				reloadLayout();
			}
		});
	});

	$effect(() => {
		const info = $repoInfo;
		const title = info?.path ? `${info.path.split('/').pop() ?? info.path} - gitv` : 'gitv';
		setWindowTitle(title).catch(() => {});
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
		if ($resolvedTheme === 'light') {
			root.classList.remove('dark');
			root.classList.add('light');
		} else {
			root.classList.add('dark');
			root.classList.remove('light');
		}
		if ($resolvedHighContrast) {
			root.classList.add('high-contrast');
		} else {
			root.classList.remove('high-contrast');
		}
		root.style.fontSize = `${$fontSize}px`;
	});

	$effect(() => {
		const cOid = $comparisonOid;
		const sOid = $selectedOid;
		void cOid;
		void sOid;
		if (!cOid || !sOid || sOid === cOid) {
			comparisonDetails = null;
			comparisonLoading = false;
			return;
		}
		comparisonDetails = null;
		comparisonLoading = true;
		getDiff(repoPath, sOid, cOid)
			.then((summary: DiffSummary) => {
				if ($comparisonOid !== cOid || $selectedOid !== sOid) return;
				comparisonDetails = {
					info: {
						oid: cOid,
						short_oid: '',
						message: '',
						summary: '',
						author: { name: '', email: '' },
						committer: { name: '', email: '' },
						author_time: '',
						commit_time: '',
						parent_oids: [],
						refs: []
					},
					tree_oid: '',
					signature: null,
					changed_files: summary.files,
					body: null
				};
				comparisonLoading = false;
			})
			.catch(() => {
				if ($comparisonOid !== cOid || $selectedOid !== sOid) return;
				comparisonDetails = null;
				comparisonLoading = false;
			});
	});

	async function onSelectCommit(oid: string, ctrlKey = false) {
		if (!ctrlKey && !compareMode && oid === $selectedOid) return;
		selectedBranch = null;
		selectedRemote = null;
		selectedTag = null;
		if (compareMode && $selectedOid && $selectedOid !== oid) {
			compareMode = false;
			comparisonOid.set(oid);
			return;
		}
		if (ctrlKey && $selectedOid && $selectedOid !== oid) {
			comparisonOid.set(oid);
			return;
		}
		compareMode = false;
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
			void loadFileCounts(oid);
		} catch {
			commitDetails = null;
		} finally {
			detailsLoading = false;
			if ($operationState === 'LoadingDetails') operationState.set('Idle');
		}
	}

	async function loadFileCounts(oid: string) {
		try {
			const counts = await getCommitFileCounts(repoPath, oid);
			if (oid !== $selectedOid || !commitDetails) return;
			const map = new Map(counts.map((c) => [c.path, c]));
			commitDetails = {
				...commitDetails,
				changed_files: commitDetails.changed_files.map((f) => {
					const c = map.get(f.path);
					return c ? { ...f, additions: c.additions, deletions: c.deletions } : f;
				})
			};
		} catch {
			// counts are optional — file list already displayed
		}
	}

	function handleWheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		announce(translate('page.cmd_font_size', { size: adjustFontSize(e.deltaY > 0 ? -1 : 1) }));
	}

	function handleKeydown(e: KeyboardEvent) {
		const target = e.target as HTMLElement | null;
		if (
			target &&
			(target.tagName === 'INPUT' ||
				target.tagName === 'TEXTAREA' ||
				target.tagName === 'SELECT' ||
				target.isContentEditable)
		) {
			if (!e.ctrlKey && !e.altKey && !e.metaKey) {
				return;
			}
		}
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
		if (e.key === 'F12' || (e.key === 'D' && (e.ctrlKey || e.metaKey) && e.shiftKey)) {
			e.preventDefault();
			toggleDebug();
			return;
		}
		if ((e.key === 'p' && e.ctrlKey) || (e.key === 'p' && e.metaKey)) {
			e.preventDefault();
			openModal('commandPalette');
			return;
		}
		if (e.key === 'm' && e.ctrlKey && !e.metaKey) {
			e.preventDefault();
			isFullscreen = !isFullscreen;
			return;
		}
		if (e.key === ',' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			openModal('preferences');
			return;
		}
		if (e.shiftKey && (e.ctrlKey || e.metaKey)) {
			if (e.key === 'M') {
				e.preventDefault();
				graphHideMerges.update((v) => {
					const next = !v;
					announce(translate(next ? 'page.merges_hidden' : 'page.cmd_toggle_merges'));
					return next;
				});
				return;
			}
			if (e.key === 'A') {
				e.preventDefault();
				graphColorMode.update((v) => {
					const next = v === 'by-branch' ? 'by-author' : 'by-branch';
					announce(
						translate('page.cmd_color_author') +
							': ' +
							(next === 'by-author'
								? translate('preferences.by_author')
								: translate('preferences.by_branch'))
					);
					return next;
				});
				return;
			}
			if (e.key === 'G') {
				e.preventDefault();
				graphOrientation.update((v) => {
					const next = v === 'top-to-bottom' ? 'bottom-to-top' : 'top-to-bottom';
					announce(
						translate('page.cmd_orientation') +
							': ' +
							(next === 'bottom-to-top'
								? translate('preferences.bottom_to_top')
								: translate('preferences.top_to_bottom'))
					);
					return next;
				});
				return;
			}
		}
		if (e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt' || e.key === 'Meta') return;
		const mod = e.ctrlKey || e.metaKey;
		if (mod && !e.altKey && ['-', '_'].includes(e.key)) {
			e.preventDefault();
			announce(translate('page.cmd_font_size', { size: adjustFontSize(-1) }));
			return;
		}
		if (mod && !e.altKey && ['=', '+'].includes(e.key)) {
			e.preventDefault();
			announce(translate('page.cmd_font_size', { size: adjustFontSize(1) }));
			return;
		}
		if (mod && !e.altKey && !e.shiftKey && e.key === '0') {
			e.preventDefault();
			announce(translate('page.cmd_font_size', { size: adjustFontSize(0) }));
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
		if (e.key === 'F1' || (e.key === '/' && (e.ctrlKey || e.metaKey))) {
			e.preventDefault();
			openModal('info');
			return;
		}
		if (e.key === 'Escape') {
			if (showCommandPalette || contextMenu || showPreferences || showInfo) return;
			if (isFullscreen) {
				isFullscreen = false;
				return;
			}
			if (compareMode) {
				compareMode = false;
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
		if (!$selectedOid || !displayedCommits.length) return;

		const currentCommit = displayedCommits.find((c) => c.oid === $selectedOid);
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
		const idx = displayedCommits.findIndex((c) => c.oid === current.oid);
		if (idx < 0) return;

		for (let i = idx + direction; i >= 0 && i < displayedCommits.length; i += direction) {
			const c = displayedCommits[i];
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
			shortcut: 'Ctrl+Shift+M',
			category: 'Graph',
			action: () => graphHideMerges.update((v) => !v)
		});
		registerCommand({
			id: 'toggle-color-mode',
			label: translate('page.cmd_color_author'),
			shortcut: 'Ctrl+Shift+A',
			category: 'Graph',
			action: () => graphColorMode.update((v) => (v === 'by-branch' ? 'by-author' : 'by-branch'))
		});
		registerCommand({
			id: 'toggle-orientation',
			label: translate('page.cmd_orientation'),
			shortcut: 'Ctrl+Shift+G',
			category: 'Graph',
			action: () =>
				graphOrientation.update((v) => (v === 'top-to-bottom' ? 'bottom-to-top' : 'top-to-bottom'))
		});
		registerCommand({
			id: 'toggle-debug',
			label: translate('page.cmd_debug'),
			shortcut: 'F12 / Ctrl+Shift+D',
			category: 'Debug',
			action: () => {
				toggleDebug();
			}
		});
		registerCommand({
			id: 'command-palette',
			label: translate('page.cmd_command_palette'),
			shortcut: modKey === '⌘' ? '⌘P' : 'Ctrl+P',
			category: 'Help',
			action: () => openModal('commandPalette')
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
			id: 'font-increase',
			label: translate('page.cmd_font_increase'),
			shortcut: modKey === '⌘' ? '⌘=' : 'Ctrl+=',
			category: 'View',
			action: () => adjustFontSize(1)
		});
		registerCommand({
			id: 'font-decrease',
			label: translate('page.cmd_font_decrease'),
			shortcut: modKey === '⌘' ? '⌘-' : 'Ctrl+-',
			category: 'View',
			action: () => adjustFontSize(-1)
		});
		registerCommand({
			id: 'font-reset',
			label: translate('page.cmd_font_reset'),
			shortcut: modKey === '⌘' ? '⌘0' : 'Ctrl+0',
			category: 'View',
			action: () => adjustFontSize(0)
		});
		registerCommand({
			id: 'compare-commits',
			label: translate('page.cmd_compare_commits'),
			category: 'View',
			action: () => {
				if ($selectedOid) compareMode = true;
			}
		});
		registerCommand({
			id: 'open-preferences',
			label: translate('page.cmd_preferences'),
			shortcut: 'Ctrl+,',
			category: 'File',
			action: () => {
				openModal('preferences');
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
			shortcut: 'F1, Ctrl+/',
			category: 'Help',
			action: () => {
				openModal('info');
			}
		});
		registerCommand({
			id: 'commit-next',
			label: translate('page.cmd_commit_next'),
			shortcut: '\u2193, J',
			category: 'Navigation',
			action: () => {}
		});
		registerCommand({
			id: 'commit-prev',
			label: translate('page.cmd_commit_prev'),
			shortcut: '\u2191, K',
			category: 'Navigation',
			action: () => {}
		});
		registerCommand({
			id: 'commit-page-down',
			label: translate('page.cmd_page_down'),
			shortcut: 'PageDown',
			category: 'Navigation',
			action: () => {}
		});
		registerCommand({
			id: 'commit-page-up',
			label: translate('page.cmd_page_up'),
			shortcut: 'PageUp',
			category: 'Navigation',
			action: () => {}
		});
		registerCommand({
			id: 'commit-first',
			label: translate('page.cmd_commit_first'),
			shortcut: 'Home',
			category: 'Navigation',
			action: () => {}
		});
		registerCommand({
			id: 'commit-last',
			label: translate('page.cmd_commit_last'),
			shortcut: 'End',
			category: 'Navigation',
			action: () => {}
		});
		registerCommand({
			id: 'author-next',
			label: translate('page.cmd_author_next'),
			shortcut: 'Alt+N',
			category: 'Navigation',
			action: () => {
				if (!$selectedOid || !displayedCommits.length) return;
				const currentCommit = displayedCommits.find((c) => c.oid === $selectedOid);
				if (currentCommit) navigateAuthor(currentCommit, 1);
			}
		});
		registerCommand({
			id: 'author-prev',
			label: translate('page.cmd_author_prev'),
			shortcut: 'Alt+P',
			category: 'Navigation',
			action: () => {
				if (!$selectedOid || !displayedCommits.length) return;
				const currentCommit = displayedCommits.find((c) => c.oid === $selectedOid);
				if (currentCommit) navigateAuthor(currentCommit, -1);
			}
		});
	}

	function handleCommitContextMenu(e: MouseEvent, oid: string) {
		e.preventDefault();
		const commit = commits.find((c) => c.oid === oid);
		const items: ContextMenuItem[] = [
			{ label: translate('page.ctx_copy_sha'), action: () => copyToClipboard(oid) },
			{
				label: translate('page.ctx_copy_short_sha'),
				action: () => copyToClipboard(oid.substring(0, 7))
			}
		];
		if (commit) {
			items.push({
				label: translate('page.ctx_copy_message'),
				action: () => copyToClipboard(commit.message)
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
				action: () => copyToClipboard(name)
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
				action: () => copyToClipboard(`${remote}/${name}`)
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
		const d = new Date(iso);
		if (isNaN(d.getTime())) return iso;
		try {
			return d.toLocaleString(get(locale), {
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
	<a
		href="#commit-list-container"
		class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-[100] focus:rounded focus:bg-blue-600 focus:px-4 focus:py-2 focus:text-sm focus:text-white"
	>
		{$t('page.skip_to_content')}
	</a>
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
					<div class="flex justify-center gap-4 text-xs text-gray-600">
						<span
							><kbd class="rounded bg-gray-800 px-1.5 py-0.5 font-mono text-[10px]">{modKey}O</kbd>
							{$t('page.browse_repo')}</span
						>
						<span
							><kbd class="rounded bg-gray-800 px-1.5 py-0.5 font-mono text-[10px]">{modKey}P</kbd>
							{$t('page.cmd_palette')}</span
						>
						<span
							><kbd class="rounded bg-gray-800 px-1.5 py-0.5 font-mono text-[10px]">F1</kbd>
							{$t('toolbar.info')}</span
						>
					</div>
					<div class="flex justify-center gap-1">
						<button
							class="rounded p-2 text-gray-400 hover:bg-gray-800 hover:text-white transition-colors"
							onclick={() => openModal('preferences')}
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
							onclick={() => openModal('info')}
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
				<div class="flex-1 min-w-0 overflow-x-auto">
					<div class="flex items-center gap-1.5 w-max">
						{#each recentRepos as repo (repo.path)}
							<button
								class={'rounded px-2 py-0.5 text-xs shrink-0 ' +
									(repo.path === repoPath
										? 'bg-blue-600 text-white'
										: 'bg-gray-700/50 text-gray-300 hover:bg-gray-600 hover:text-gray-100')}
								onclick={() => {
									repoPath = repo.path;
									loadRepo(repo.path);
								}}
								title={repo.path}
							>
								{repo.name}
							</button>
						{/each}
					</div>
				</div>
				<div class="shrink-0">
					<Toolbar
						onrefresh={manualRefresh}
						onopensettings={() => openModal('preferences')}
						onopeninfo={() => openModal('info')}
					/>
				</div>
				{#if graphLayout}
					<div class="shrink-0">
						<AuthorLegend layout={graphLayout} />
					</div>
				{/if}
				<div class="ml-auto flex items-center gap-3">
					{#key repoPath}
						<SearchBar {repoPath} />
					{/key}
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
				<ResizeHandle
					direction="horizontal"
					forSidebar
					bind:sidebarWidth
					minWidth={150}
					maxWidth={512}
					onDragEnd={() => updateLayout({ sidebarWidth })}
				/>
			{/if}
			<div
				class="flex-1 min-h-0 overflow-hidden flex flex-col"
				role="main"
				aria-label={$t('commit_list.aria')}
			>
				<h1 class="sr-only">{$repoInfo?.path ?? $t('page.repository')}</h1>
				<div id="commit-list-container" class="flex-1 min-h-0 overflow-hidden">
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
								{rowHeight}
								onLoadMore={loadMoreCommits}
								loadedCommitCount={commits.length}
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
					{:else}
						<div class="flex h-full items-center justify-center" role="status">
							<span class="text-sm text-gray-500">{$t('page.no_commits')}</span>
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
							{#if comparisonLoading}
								<div
									class="flex items-center justify-center h-full text-sm text-gray-500"
									role="status"
									aria-live="polite"
								>
									{$t('comparison.loading')}
								</div>
							{:else if comparisonDetails}
								<CommitDetailPanel
									details={comparisonDetails}
									{repoPath}
									comparisonFromOid={$selectedOid}
									comparisonToOid={$comparisonOid}
									onswap={() => {
										const tmp = $selectedOid;
										selectedOid.set($comparisonOid);
										comparisonOid.set(tmp);
									}}
									onclose={() => comparisonOid.set(null)}
								/>
							{:else}
								<div
									class="flex items-center justify-center h-full text-sm text-red-400"
									role="alert"
								>
									{$t('comparison.failed')}
								</div>
							{/if}
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
								{patchMatches}
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
	{#if $repoInfo && $operationState !== 'LoadingRepo'}
		<footer
			class="flex items-center gap-3 border-t border-gray-800 px-4 py-1 text-xs text-gray-500 shrink-0"
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
			{#if compareMode}
				<span class="rounded bg-blue-700/50 px-2 py-0.5 text-xs text-blue-300">
					{$t('page.compare_mode')}
				</span>
			{/if}
			{#if $comparisonOid}
				<span class="rounded bg-indigo-700/50 px-2 py-0.5 text-xs text-indigo-300">
					{$t('page.comparing')}
				</span>
			{/if}
			{#if $patchSearchActive && $patchSearchProgress}
				<span class="rounded bg-cyan-700/50 px-2 py-0.5 text-xs text-cyan-300">
					{$t('search.patch_progress', $patchSearchProgress)}
				</span>
			{/if}
			{#if $operationState !== 'Idle'}
				{@const opKey = 'page.' + $operationState.toLowerCase()}
				{@const opLabel = $t(opKey)}
				<span class="text-blue-400" aria-live="polite">
					{opLabel === opKey ? $operationState : opLabel}
				</span>
			{/if}
		</footer>
	{/if}
	<ToastContainer />
	<DebugOverlay {repoPath} />
	{#if showPreferences}
		<PreferencesModal
			onclose={() => {
				showPreferences = false;
				restoreFocus();
			}}
		/>
	{/if}
	{#if showInfo}
		<InfoDialog
			onclose={() => {
				showInfo = false;
				restoreFocus();
			}}
		/>
	{/if}
	{#if showCommandPalette}
		<CommandPalette
			onclose={() => {
				showCommandPalette = false;
				restoreFocus();
			}}
		/>
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
