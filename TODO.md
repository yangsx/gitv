# TODO — Unimplemented Requirements

Requirements from `requirements.md` that are not yet implemented, organized by priority.

## High Priority

### Req 55: CLI Revision Ranges and Filter Flags
Implement `gitv <repo> [<revision-range>]` with `--branches`, `--author`, `-- <path>` flags.
- Single revision, double-dot, triple-dot range formats
- Pre-apply filter on open; fall back to full history on invalid range
- See: `src-tauri/src/cli.rs`, `crates/gitv-git-core/src/repository.rs`

### Req 17: Revision Range Selection
UI to select start/end commits defining a range; display only commits in range.
- Select by branch names, tags, or commit SHAs
- Visual range boundaries on graph
- Quick-select "commits since last tag"
- Exclude specific commits (negation)

### Req 35: Two-Commit Comparison (Multi-Selection)
- Ctrl+Click / Cmd+Click to select a second commit in CommitList or graph
- Context menu "Compare with selected" action
- Visual distinction for compared commits
- Escape clears comparison
- ComparisonPanel.svelte exists (AC 35.3) but lacks multi-select affordances

### Req 41: Saved Searches — Frontend UI
Backend `save_search`/`list_saved_searches`/`delete_saved_search` exist. Missing:
- Save button in SearchBar
- Saved search dropdown with apply/delete
- See: `frontend/src/lib/components/SearchBar.svelte`

### Req 61: Large Diff Lazy Loading
Load file list + stats first, load individual file diffs on demand.
- Per-file line limit (default 10k) with "Show full diff" affordance
- Loading spinners per file
- Total changed-files count in diff header

## Medium Priority

### Req 16: Line-Level History Tracing
Select a line range or function name, view commit history for those lines.
- `git log -L` equivalent
- Trace multiple ranges simultaneously
- Absolute line numbers and regex pattern modes

### Req 18: Merge Conflict Visualization
Detect merge conflicts, display conflict-relevant commits on each side.
- Left/right markers for side attribution
- Conflict markers with both versions

### Req 19: Advanced Ref Filtering
Filter commits by branch/tag/remote glob patterns, combine/exclude.

### Req 20: Ancestry Path Filtering
Specify two commits, view only the ancestry path between them.

### Req 4.5–4.7: Graph Zoom and Scrolling
- Zoom in/out while maintaining readable commit info
- Horizontal/vertical scrolling for large graphs
- Must work in both Canvas 2D and wgpu renderers

### Req 26.3: Back/Forward Navigation History
Track commit selection, scroll position, filter state changes (min 50 steps).

### Req 26.4: Graph Mini-Map
Navigable overview for large commit graphs.

### Req 63: Mid-Stream Error Recovery
Retain commits received before stream failure; retry button; per-file diff error handling.

## Low Priority

### Req 21: Localization — Additional Languages
- Japanese, German, Spanish, French translation files (architecture now supports discovery — just drop a JSON in `locales/`)
- Locale-aware date/time/number formatting
- RTL layout support

### Req 7.5: File Content at Historical Commit
`blob_content` and `showBlob` exist but file-at-historical-commit viewing needs a dedicated UI affordance.

### Req 48.5: Summary/Body Distinction in Commit List
Show summary line separately from body text in expanded commit list rows.

### Req 46.4: Keyboard Shortcuts for All Context Menu Actions
Ensure every context menu action (copy SHA, copy path, view file history, etc.) has a keyboard shortcut.

### Req 36.1: File History `--follow` Toggle
Explicit enable/disable for rename following in file history.

### Req 64.1, 64.5: Submodule Display
- Show pinned commit SHA in file tree submodule entries
- Context menu actions for submodules ("Copy submodule path", "Copy pinned commit SHA")

### Req 58: Navigation by Author
Jump to next/previous commit by same author via keyboard shortcuts.

## Observations

- **Req 6.5 (date range), 6.6 (file path), 25.6 (stream cancellation), 44 (file tree browser), 59 (panel layout persistence)** — all ARE implemented despite initial uncertainty.
- **Req 11.2 (responsive layout)** — panels are resizable but layout is fixed-format. Minor.
- **Req 25.5 (viewport-prioritized streaming)** — CommitStream loads all commits upfront then serves batches; viewport-aware loading would reduce initial latency for large repos.
- **Req 21.7 (locale formatting)** — browser `toLocaleDateString()` uses system locale, not app-selected locale.
