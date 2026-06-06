# Requirements Document

## Introduction

gitv is a modern cross-platform Git visualization tool built with Rust and Tauri. It serves as a contemporary reimplementation of gitk, providing users with an intuitive graphical interface for exploring Git repositories. Unlike gitk, which is typically launched from within a repository, gitv allows users to launch the application independently and open any Git project directory.

## Glossary

- **gitv**: The application being developed - a modern Git visualization tool
- **Repository**: A Git repository containing version-controlled files and commit history
- **Commit**: A snapshot of the repository at a specific point in time
- **Branch**: A named reference to a line of development
- **Commit Graph**: A visual representation of commit history showing parent-child relationships
- **Diff**: The difference between two versions of a file or between commits
- **Working Directory**: The current directory the user has opened in gitv
- **HEAD**: The currently checked-out commit or branch
- **Remote**: A reference to a remote repository (e.g., origin)
- **Staged Changes**: Changes added to the index and ready to be committed

## Requirements

### Requirement 1: Application Launch and Repository Opening

**User Story:** As a developer, I want to launch gitv independently and open any Git repository, so that I can visualize projects without navigating to their directories first.

#### Acceptance Criteria

1. WHEN the gitv application is launched and no repository path is provided via CLI or URL, THE gitv_Application SHALL display a welcome screen with an option to open a repository
2. WHEN the welcome screen is shown and the recent repositories list is empty, THE native file picker dialog SHALL open automatically (no button click needed); if the dialog is cancelled, A "Browse for Repository…" button SHALL be shown as fallback
3. WHEN the recent repositories list is non-empty, THE welcome screen SHALL display the list prominently; THE user MAY click a recent entry or the "Browse for Repository…" button
4. WHEN a user selects a valid Git repository directory (via native dialog, drag-and-drop, CLI arg, or recent repo), THE gitv_Application SHALL load and display the repository
5. WHEN a user selects a non-Git directory, THE gitv_Application SHALL display a localized, informative error message (e.g., "{path} is not a Git repository") without leaking implementation details (e.g., "gix error")
6. THE gitv_Application SHALL remember the last 10 opened repositories, persisted to $XDG_CONFIG_HOME/gitv/recent_repos.json with canonicalized paths and atomic writes
7. WHEN a user selects a recently opened repository, THE gitv_Application SHALL load that repository directly
8. WHEN the recent repositories list is non-empty, THE welcome screen SHALL display them as clickable entries showing repository name, path, and last-opened timestamp
9. WHEN the recent repositories list is empty, THE welcome screen SHALL display only the "Browse for Repository…" button with a message inviting the user to open their first repository
10. WHEN a repository is already open, THE toolbar SHALL display a "Browse for Repository…" button and clickable chips for each recent repository, allowing the user to open without navigating to the welcome screen
11. WHEN a user selects a subdirectory within a Git repository (e.g., via native dialog or drag-and-drop), THE gitv_Application SHALL discover and use the repository root path, not the subdirectory
12. THE gitv_Application SHALL support Ctrl+W / Cmd+W to close the current repository and return to the welcome screen, with all per-repository state cleared
13. THE command palette SHALL list recent repositories under a "Recent" category, allowing the user to open any recent repo by name

### Requirement 2: Commit History Visualization

**User Story:** As a developer, I want to see the commit history of my repository, so that I can understand the evolution of the project.

#### Acceptance Criteria

1. WHEN a repository is loaded, THE gitv_Application SHALL display the commit history in a scrollable list
2. THE Commit_Graph SHALL display each commit with its SHA, author, date, and commit message
3. WHEN a commit is selected, THE gitv_Application SHALL display the commit details in a detail panel
4. THE gitv_Application SHALL support sorting commits by date, author, or SHA
5. THE gitv_Application SHALL support filtering commits by author, date range, or commit message text
6. WHEN the commit history exceeds 1000 commits, THE gitv_Application SHALL implement virtual scrolling for performance

### Requirement 3: Branch Visualization

**User Story:** As a developer, I want to see all branches and their relationships, so that I can understand the project's development structure.

#### Acceptance Criteria

1. WHEN a repository is loaded, THE gitv_Application SHALL display all branches in a sidebar panel
2. THE Commit_Graph SHALL visually represent branch relationships with colored lines
3. WHEN a branch is selected, THE gitv_Application SHALL highlight all commits belonging to that branch
4. THE gitv_Application SHALL display the current HEAD branch with a distinct visual indicator
5. THE gitv_Application SHALL distinguish between local and remote branches
6. THE gitv_Application SHALL display merged branches with a different visual style from active branches

### Requirement 4: Commit Graph Display

**User Story:** As a developer, I want to see a graphical representation of commit history, so that I can understand branching and merging at a glance.

#### Acceptance Criteria

1. WHEN a repository is loaded, THE gitv_Application SHALL display a graphical commit graph
2. THE Commit_Graph SHALL draw nodes for each commit and edges for parent-child relationships
3. THE Commit_Graph SHALL use different colors for different branches
4. WHEN a commit node is hovered, THE gitv_Application SHALL display a tooltip with commit summary
5. THE Commit_Graph SHALL support zoom in and zoom out functionality
6. THE Commit_Graph SHALL support horizontal and vertical scrolling for large graphs
7. WHEN the graph is zoomed, THE gitv_Application SHALL maintain readable commit information

### Requirement 5: Diff Viewing

**User Story:** As a developer, I want to view the differences between commits, so that I can understand what changed.

#### Acceptance Criteria

1. WHEN a commit is selected, THE gitv_Application SHALL display the list of changed files
2. WHEN a file is selected from the changed files list, THE gitv_Application SHALL display the diff in a diff viewer
3. THE Diff_Viewer SHALL display additions in green and deletions in red
4. THE Diff_Viewer SHALL support unified diff format display
5. THE Diff_Viewer SHALL support side-by-side diff view mode
6. WHEN comparing two commits, THE gitv_Application SHALL display the combined diff between them
7. THE Diff_Viewer SHALL display line numbers for both the old and new versions

### Requirement 6: Search and Filtering

**User Story:** As a developer, I want to search and filter the commit history, so that I can find specific changes.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a search bar for commit messages
2. WHEN a search term is entered, THE gitv_Application SHALL filter commits to those matching the term
3. THE gitv_Application SHALL support searching by commit SHA (full or partial)
4. THE gitv_Application SHALL support searching by author name or email
5. THE gitv_Application SHALL support filtering by date range
6. THE gitv_Application SHALL support filtering by file path (showing commits that modified a specific file)
7. WHEN filters are active, THE gitv_Application SHALL display the number of matching commits
8. THE gitv_Application SHALL support searching within commit diffs/patches (added and removed lines)
9. THE gitv_Application SHALL support searching for code patterns across all commits in the repository
10. THE gitv_Application SHALL allow users to combine commit message search with diff search
11. WHEN a diff search is performed, THE gitv_Application SHALL highlight the matching lines within the diff viewer
12. THE gitv_Application SHALL support regex pattern matching in diff searches

### Requirement 7: File History (Git Blame)

**User Story:** As a developer, I want to see the history of a specific file, so that I can understand how it evolved.

#### Acceptance Criteria

1. WHEN a file is selected from the file browser, THE gitv_Application SHALL display the file's commit history
2. THE gitv_Application SHALL display blame information showing which commit modified each line
3. WHEN a blame line is clicked, THE gitv_Application SHALL navigate to that commit
4. THE gitv_Application SHALL display the author and date for each blamed line
5. THE gitv_Application SHALL support viewing the file contents at any historical commit

### Requirement 8: Tag Visualization

**User Story:** As a developer, I want to see tags in the repository, so that I can identify important release points.

#### Acceptance Criteria

1. THE gitv_Application SHALL display all tags in the repository
2. THE Commit_Graph SHALL display tags as visual markers on their associated commits
3. WHEN a tag is selected, THE gitv_Application SHALL highlight the associated commit
4. THE gitv_Application SHALL display the tag name and associated commit information

**Note:** All operations in gitv are read-only with respect to the Git repository. Tag management operations (create, delete, modify) are out of scope for this visualization tool.

### Requirement 9: Cross-Platform Support

**User Story:** As a developer, I want gitv to work on my operating system, so that I can use it regardless of my platform.

#### Acceptance Criteria

1. THE gitv_Application SHALL run on Windows 10 and later
2. THE gitv_Application SHALL run on macOS 10.15 (Catalina) and later
3. THE gitv_Application SHALL run on Linux distributions supporting GTK 3 or later
4. THE gitv_Application SHALL use native file dialogs on each platform
5. THE gitv_Application SHALL follow platform-specific UI conventions and keyboard shortcuts
6. THE gitv_Application SHALL handle platform-specific path separators correctly

### Requirement 10: Performance

**User Story:** As a developer, I want gitv to be responsive, so that I can work efficiently even with large repositories.

#### Acceptance Criteria

1. WHEN opening a repository with 100,000+ commits, THE gitv_Application SHALL load within 5 seconds
2. WHEN scrolling through commit history, THE gitv_Application SHALL maintain 60 frames per second
3. WHEN searching commits (indexed), THE gitv_Application SHALL return results within 100 milliseconds
4. THE gitv_Application SHALL use background threads for Git operations to keep the UI responsive
5. THE gitv_Application SHALL implement lazy loading for commit details

### Requirement 11: Modern GUI

**User Story:** As a developer, I want a modern and intuitive user interface, so that I can efficiently navigate and understand my repository.

#### Acceptance Criteria

1. THE gitv_Application SHALL use a dark theme by default with a light theme option
2. THE gitv_Application SHALL use a responsive layout with resizable panels
3. THE gitv_Application SHALL display keyboard shortcuts for common actions
4. THE gitv_Application SHALL support keyboard navigation throughout the interface
5. THE gitv_Application SHALL provide a status bar showing repository information
6. THE gitv_Application SHALL support customizable font sizes for commit messages and diffs
7. THE gitv_Application SHALL provide a preferences/settings button accessible from both the welcome screen and the main layout
8. THE settings modal SHALL include: theme selection, font size, keyboard shortcut customization, and language selection

### Requirement 12: Repository Status Display

**User Story:** As a developer, I want to see the current state of my working directory, so that I know what changes I have pending.

#### Acceptance Criteria

1. WHEN a repository is loaded, THE gitv_Application SHALL display the current branch name (or "(detached)" with the short commit SHA when HEAD is detached)
2. THE gitv_Application SHALL display the number of uncommitted changes (staged and unstaged)
3. THE gitv_Application SHALL indicate if the current branch is ahead or behind its remote
4. THE gitv_Application SHALL display a summary of staged and unstaged files
5. THE gitv_Application SHALL refresh the status when changes are detected in the repository

### Requirement 13: Keyboard Navigation

**User Story:** As a developer, I want to navigate the interface using keyboard shortcuts, so that I can work more efficiently.

#### Acceptance Criteria

1. THE gitv_Application SHALL support keyboard shortcuts for common actions (open, search, filter)
2. THE gitv_Application SHALL support navigating between commits using arrow keys
3. THE gitv_Application SHALL support jumping to the next/previous branch in the commit list
4. THE gitv_Application SHALL display a keyboard shortcut help dialog
5. THE gitv_Application SHALL support customizable keyboard shortcuts

### Requirement 14: Accessibility

**User Story:** As a developer with accessibility needs, I want gitv to be usable with assistive technologies, so that I can effectively use the application.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide proper accessibility labels for all interactive elements
2. THE gitv_Application SHALL support screen readers on each platform
3. THE gitv_Application SHALL support high contrast themes
4. THE gitv_Application SHALL be fully navigable using only a keyboard
5. THE gitv_Application SHALL respect system accessibility settings

### Requirement 15: Error Handling

**User Story:** As a developer, I want meaningful error messages, so that I can understand and resolve issues.

#### Acceptance Criteria

1. WHEN a Git operation fails, THE gitv_Application SHALL display a user-friendly error message
2. WHEN a repository is corrupted, THE gitv_Application SHALL display diagnostic information
3. WHEN a network operation fails (for remote operations), THE gitv_Application SHALL provide retry options
4. THE gitv_Application SHALL log errors to a file for troubleshooting
5. WHEN an unexpected error occurs, THE gitv_Application SHALL recover gracefully without crashing

### Requirement 16: Line-Level History Tracing

**User Story:** As a developer, I want to trace the evolution of specific lines or functions within a file, so that I can understand how a particular piece of code changed over time.

#### Acceptance Criteria

1. THE gitv_Application SHALL allow users to select a line range within a file and view its commit history
2. THE gitv_Application SHALL support tracing the evolution of a function by its name (regex pattern)
3. WHEN a line range is selected for tracing, THE gitv_Application SHALL display all commits that modified those lines
4. THE gitv_Application SHALL display the diff for each commit focused on the selected line range
5. THE gitv_Application SHALL support specifying line numbers as absolute positions or regex patterns
6. THE gitv_Application SHALL allow multiple line ranges to be traced simultaneously

### Requirement 17: Revision Range Selection

**User Story:** As a developer, I want to view commits between two specific points in history, so that I can focus on a specific segment of the project's evolution.

#### Acceptance Criteria

1. THE gitv_Application SHALL allow users to select a start and end commit to define a revision range
2. WHEN a revision range is selected, THE gitv_Application SHALL display only commits within that range
3. THE gitv_Application SHALL support selecting revision ranges by branch names, tags, or commit SHAs
4. THE gitv_Application SHALL display a visual indicator showing the selected range boundaries on the commit graph
5. THE gitv_Application SHALL allow users to quickly select "commits since last tag" or "commits between two tags"
6. THE gitv_Application SHALL support excluding specific commits from the view (negation)

### Requirement 18: Merge Conflict Visualization

**User Story:** As a developer dealing with merge conflicts, I want to see which commits modified the conflicted files, so that I can understand the context and resolve conflicts more easily.

#### Acceptance Criteria

1. WHEN a repository has unresolved merge conflicts, THE gitv_Application SHALL detect and indicate this state
2. THE gitv_Application SHALL display commits that modified conflicted files on both branches (HEAD and MERGE_HEAD)
3. THE gitv_Application SHALL highlight which commits are unique to each side of the merge
4. THE gitv_Application SHALL provide a filtered view showing only commits relevant to the conflict
5. THE gitv_Application SHALL display left/right markers indicating which side each commit belongs to
6. WHEN viewing conflicted files, THE gitv_Application SHALL display the conflict markers and both versions

### Requirement 19: Advanced Ref Filtering

**User Story:** As a developer, I want to filter commits by specific ref patterns, so that I can focus on relevant branches, tags, or remotes.

#### Acceptance Criteria

1. THE gitv_Application SHALL support filtering commits by branch name patterns (glob patterns)
2. THE gitv_Application SHALL support filtering commits by tag name patterns
3. THE gitv_Application SHALL support filtering commits by remote name patterns
4. THE gitv_Application SHALL provide an option to show all refs (branches, tags, remotes) simultaneously
5. THE gitv_Application SHALL allow combining multiple ref filters
6. THE gitv_Application SHALL support excluding specific refs from the view

### Requirement 20: Ancestry Path Filtering

**User Story:** As a developer, I want to see only the commits on the direct ancestry chain between two commits, so that I can understand the exact path of changes from one point to another.

#### Acceptance Criteria

1. THE gitv_Application SHALL allow users to specify two commits and view only the ancestry path between them
2. WHEN ancestry path filtering is active, THE gitv_Application SHALL hide commits not on the direct path
3. THE gitv_Application SHALL display the ancestry path as a highlighted route on the commit graph
4. THE gitv_Application SHALL support showing full history with ancestry path highlighted as an alternative view mode

### Requirement 21: Localization Support

**User Story:** As a developer who speaks a language other than English, I want gitv to display its interface in my preferred language, so that I can use the application comfortably in my native language.

#### Acceptance Criteria

1. THE gitv_Application SHALL support multiple languages for its user interface
2. THE gitv_Application SHALL detect the system language on first launch and use it if supported
3. THE gitv_Application SHALL allow users to change the display language from the settings
4. THE gitv_Application SHALL include English as the default language
5. THE gitv_Application SHALL support the following languages at minimum: English, Simplified Chinese, Japanese, German, Spanish, French
6. THE gitv_Application SHALL use UTF-8 encoding for all localized text to support non-Latin scripts
7. THE gitv_Application SHALL format dates, times, and numbers according to the selected locale
8. THE gitv_Application SHALL support right-to-left (RTL) layout for languages like Arabic and Hebrew
9. THE gitv_Application SHALL display language-specific keyboard shortcuts where applicable
10. THE gitv_Application SHALL allow the community to contribute additional language translations

### Requirement 22: Manual Refresh

**User Story:** As a developer, I want to refresh the displayed data when I choose, so that I can see the latest repository state on demand.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a refresh button in the toolbar
2. WHEN the refresh button is clicked, THE gitv_Application SHALL reload commits, the graph layout, refs, and working changes
3. THE gitv_Application SHALL display a loading indicator while the refresh is in progress
4. THE gitv_Application SHALL show a toast with the commit count after refresh completes

### Requirement 23: Architecture - Git Logic Decoupling

**User Story:** As a developer of gitv, I want the Git logic to be decoupled from the UI layer, so that the codebase is maintainable, testable, and the Git backend could be reused independently.

#### Acceptance Criteria

1. THE gitv_Application SHALL separate Git operations into a distinct backend module with no UI dependencies
2. THE Git backend module SHALL expose a clean API that returns data structures without UI coupling
3. THE UI layer SHALL consume the Git backend through well-defined interfaces
4. THE Git backend SHALL be independently testable with unit tests
5. THE Git backend SHALL NOT depend on Tauri-specific APIs
6. THE architecture SHALL allow for potential future replacement of the Git backend implementation

### Requirement 24: High-Performance Commit Graph Rendering

**User Story:** As a developer working with large repositories, I want the commit graph to render smoothly and quickly, so that I can navigate history efficiently even with thousands of commits.

#### Acceptance Criteria

1. THE Commit_Graph SHALL use GPU-accelerated rendering for optimal performance
2. THE Commit_Graph SHALL implement virtualized rendering to only draw visible commits
3. THE Commit_Graph SHALL maintain 60 frames per second during pan and zoom operations
4. THE Commit_Graph SHALL cache rendered elements to minimize redraws
5. THE Commit_Graph SHALL use efficient data structures for graph layout calculations
6. THE Commit_Graph SHALL render incrementally for large graphs, showing progress during initial load

### Requirement 25: Streaming Git Data

**User Story:** As a developer working with large repositories, I want gitv to stream Git data progressively instead of loading everything at once, so that I can start exploring the repository immediately without waiting for a full load.

#### Acceptance Criteria

1. THE gitv_Application SHALL stream commit data from Git in batches rather than loading all at once
2. THE gitv_Application SHALL display commits as they are streamed, allowing users to browse immediately
3. THE gitv_Application SHALL continue loading data in the background while the UI remains interactive
4. THE gitv_Application SHALL show a loading progress indicator during initial data fetch
5. THE gitv_Application SHALL prioritize loading commits visible in the current viewport
6. THE gitv_Application SHALL support canceling ongoing data streams if the user navigates away

### Requirement 26: Modern UX Improvements

**User Story:** As a developer, I want gitv to provide modern UX features that gitk lacks, so that I have a more efficient and pleasant experience.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide smooth animations for transitions and state changes
2. THE gitv_Application SHALL display contextual tooltips with helpful information
3. THE gitv_Application SHALL support back/forward navigation history, maintaining a history of at least 50 steps covering: commit selection changes, scroll position jumps, and filter state changes
4. THE gitv_Application SHALL provide a mini-map for navigating large commit graphs
5. THE gitv_Application SHALL support split-view for comparing two commits side by side
6. THE gitv_Application SHALL remember window size, position, and panel layouts between sessions
7. THE gitv_Application SHALL provide a welcome screen that displays inline keyboard shortcut hints for the 3-5 most common actions (e.g., open repository, command palette, search), using platform-appropriate modifier keys (Ctrl on Windows/Linux, Cmd on macOS), visually subordinate to primary actions
8. THE gitv_Application SHALL support pinning commits for quick reference during a session

### Requirement 31: Multi-Instance Repository Opening

**User Story:** As a developer, I want to open multiple repositories in separate windows, so that I can view them side by side or on different monitors.

#### Acceptance Criteria

1. WHEN a new repository path is passed via CLI, THE gitv_Application SHALL open it in a new independent window (not a tab within the existing window)
2. Each window SHALL have isolated state — its own stores, scroll position, selected commit, filters, and sidebar state
3. Multiple windows SHALL share the same persistent configuration (preferences, recent repositories cache)
4. THE gitv_Application SHALL NOT enforce single-instance behavior
5. THE gitv_Application SHALL provide a keyboard shortcut (Ctrl+Shift+O / Cmd+Shift+O) to open a repository in a new window
6. THE command palette SHALL provide an "Open Repository in New Window" command

### Requirement 32: Focused Branch View

**User Story:** As a developer, I want to view commits from a single branch, so that I can focus on my work without distraction from other branches.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide an option to show only commits from the selected branch
2. THE gitv_Application SHALL display a toggle to switch between "all branches" and "selected branch" view
3. WHEN in single branch mode, THE gitv_Application SHALL show only first-parent commits by default
4. THE gitv_Application SHALL allow toggling first-parent-only filter independently

### Requirement 33: Fullscreen Mode

**User Story:** As a developer, I want to maximize the commit history or diff view, so that I can see more content when needed.

#### Acceptance Criteria

1. THE gitv_Application SHALL support fullscreen mode for the history view (Ctrl+M or Cmd+M)
2. THE gitv_Application SHALL support fullscreen mode for the diff view
3. WHEN in fullscreen mode, THE gitv_Application SHALL hide non-essential UI elements
4. THE gitv_Application SHALL allow exiting fullscreen mode via Escape key or keyboard shortcut toggle

### Requirement 34: File Tree Search

**User Story:** As a developer, I want to search within the file tree view, so that I can quickly find files in repositories with many files.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a search input in the file tree view
2. WHEN a search term is entered, THE gitv_Application SHALL filter the file tree to matching files
3. THE gitv_Application SHALL support fuzzy matching for file tree search
4. THE gitv_Application SHALL preserve the search filter when navigating between commits

### Requirement 27: Performance Benchmarking

**User Story:** As a developer of gitv, I want performance benchmarks to ensure the application meets its performance targets, so that we can detect regressions and validate optimizations.

#### Acceptance Criteria

1. THE gitv_Project SHALL include automated performance benchmarks for key operations
2. THE benchmarks SHALL measure repository loading time for various repository sizes (1k, 10k, 100k commits)
3. THE benchmarks SHALL measure UI responsiveness metrics (scrolling FPS, search latency)
4. THE benchmarks SHALL measure memory usage during typical operations
5. THE benchmarks SHALL be runnable in CI to detect performance regressions
6. THE benchmarks SHALL produce reports comparing against baseline performance targets

### Requirement 28: Commit Graph and List Alignment

**User Story:** As a developer familiar with gitk, I want the commit graph to remain aligned with the commit list like gitk does, so that I can easily correlate graph nodes with list entries.

#### Acceptance Criteria

1. THE gitv_Application SHALL display the commit graph and commit list side by side with synchronized scrolling
2. THE commit graph nodes SHALL be vertically aligned with their corresponding commit list rows
3. WHEN a commit is selected in either the graph or the list, THE gitv_Application SHALL highlight it in both views
4. WHEN scrolling either the graph or the list, THE gitv_Application SHALL synchronize the scroll position of the other view
5. THE alignment SHALL be maintained during zoom operations (graph zoom affects row height proportionally)
6. THE gitv_Application SHALL maintain the dense, information-rich layout style of gitk

### Requirement 29: Command Palette

**User Story:** As a developer, I want a command palette for quick access to all features, so that I can navigate and execute actions efficiently without memorizing keyboard shortcuts.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a command palette accessible via a keyboard shortcut
2. THE command palette SHALL support fuzzy search for commands and actions
3. THE command palette SHALL list all available actions with their keyboard shortcuts
4. THE command palette SHALL support navigation to specific commits by SHA or message search
5. THE command palette SHALL allow opening recent repositories by name under a "Recent" category
6. THE command palette SHALL provide commands for opening, closing, and opening-in-new-window repository operations

### Requirement 30: Pure Rust Implementation

**User Story:** As a developer, I want gitv to be implemented in pure Rust where possible, so that the codebase is safe, performant, and easy to build without external C dependencies.

#### Acceptance Criteria

1. THE gitv_Application SHALL use Rust for all application code
2. THE gitv_Application SHALL prefer pure-Rust crates over bindings to C libraries when viable options exist
3. THE Git backend SHALL use a pure-Rust Git library (e.g., gitoxide) if it provides sufficient functionality
4. IF a C dependency is unavoidable (e.g., for platform-specific features), THE gitv_Project SHALL clearly document the dependency and rationale
5. THE build process SHALL NOT require installing external C libraries for core functionality on any supported platform
6. THE gitv_Project SHALL document any exceptions to pure-Rust with justification

---

> Requirements 35-41 were added during architecture review for performance, power-user history exploration, and lightweight local-first operation.

### Requirement 35: Two-Commit Comparison

**User Story:** As a developer, I want to select any two commits and see the combined diff between them, so that I can understand exactly what changed between two arbitrary points in history.

#### Acceptance Criteria

1. THE gitv_Application SHALL allow selecting two commits via Ctrl+Click (or Cmd+Click on macOS) in the commit list or graph
2. THE gitv_Application SHALL allow selecting a second commit via right-click context menu "Compare with selected"
3. WHEN two commits are selected for comparison, THE gitv_Application SHALL display a comparison panel showing the combined diff
4. THE gitv_Application SHALL visually indicate which two commits are selected (distinct highlight from single selection)
5. THE gitv_Application SHALL allow clearing the comparison via Escape key

### Requirement 36: File History with Rename Following

**User Story:** As a developer, I want to trace a file's history across renames, so that I can see the full evolution of code even when files were moved or renamed.

#### Acceptance Criteria

1. THE gitv_Application SHALL support `--follow` mode when viewing file history
2. WHEN a file was renamed, THE file history SHALL continue tracing the file under its previous name
3. THE file history SHALL indicate when a rename occurred, showing both old and new paths
4. THE `--follow` mode SHALL be enabled by default for file history views

### Requirement 37: Reflog Visualization

**User Story:** As a developer, I want to see the reflog entries for my repository, so that I can find and navigate to commits that may no longer be reachable from any branch (e.g., after a rebase or reset).

#### Acceptance Criteria

1. THE gitv_Application SHALL display reflog entries in a dedicated panel accessible from the sidebar
2. THE reflog panel SHALL show each entry's commit SHA, operation description, author, and timestamp
3. WHEN a reflog entry is clicked, THE gitv_Application SHALL navigate to that commit in the graph
4. THE gitv_Application SHALL support viewing reflog for HEAD by default, with a dropdown to select other refs
5. THE reflog panel SHALL support filtering by operation type (commit, rebase, reset, checkout, etc.)

### Requirement 38: Stash Browsing

**User Story:** As a developer, I want to see each stash displayed as a single, clearly marked entry in the commit graph (not gitk's two-node double-diff display), so that I can quickly understand what I stashed and where without confusion.

#### Acceptance Criteria

1. THE commit graph SHALL display each stash as a single visual marker (stash icon/badge) placed on the row of the commit the stash was created from (its parent commit)
2. Stash markers SHALL be visually distinct from branch labels, tag labels, and regular commit nodes (unique icon, color, or shape)
3. WHEN a stash marker is clicked, THE gitv_Application SHALL display a combined diff in the detail panel (equivalent to `git stash show -p`), using standard +/- diff markers (not gitk's double +/−)
4. THE detail panel SHALL provide a toggle to split the stash diff into "staged changes" and "unstaged changes" for users who need that distinction
5. THE toolbar SHALL provide a toggle to show or hide all stash markers in the graph
6. THE sidebar SHALL display a stash list as a secondary navigation aid — clicking a stash entry SHALL scroll the graph to the corresponding marker
7. THE stash list SHALL show each stash's index, message, and timestamp
8. WHEN a stash marker is hovered, THE gitv_Application SHALL display a tooltip with the stash message and a summary of changed files

### Requirement 39: Persistent Repository Cache

**User Story:** As a developer who opens the same repositories daily, I want gitv to re-open previously visited repositories near-instantly, so that I don't wait for full re-traversal every time.

#### Acceptance Criteria

1. THE gitv_Application SHALL cache computed graph layout and commit metadata to disk
2. WHEN a cached repository is re-opened and no refs have changed, THE gitv_Application SHALL load from cache in < 200ms
3. WHEN refs have changed since cache, THE gitv_Application SHALL compute only the incremental diff and update the cache
4. THE cache SHALL be stored in the platform-appropriate data directory
5. THE cache SHALL be evicted by LRU policy when exceeding 20 cached repositories
6. THE gitv_Application SHALL gracefully fall back to full traversal if the cache is corrupted

### Requirement 40: Application Startup Performance

**User Story:** As a developer, I want gitv to launch quickly and feel lightweight, so that it's a tool I reach for instinctively.

#### Acceptance Criteria

1. THE gitv_Application SHALL display the welcome screen within 500ms of launch on a modern machine
2. THE gitv_Application binary SHALL be under 15MB (excluding the system webview runtime)
3. THE gitv_Application SHALL have zero network dependency for all core functionality
4. THE gitv_Application SHALL use binary serialization for commit batch streaming to minimize IPC overhead

### Requirement 41: Saved Searches

**User Story:** As a developer, I want to save frequently used search queries, so that I can quickly re-run common history investigations.

#### Acceptance Criteria

1. THE gitv_Application SHALL allow saving the current search query with a user-defined name
2. THE gitv_Application SHALL display saved searches in a dropdown accessible from the search bar
3. WHEN a saved search is selected, THE gitv_Application SHALL apply the saved query immediately
4. THE gitv_Application SHALL persist saved searches between sessions
5. THE gitv_Application SHALL allow deleting saved searches

### Requirement 42: CLI Repository Opening

**User Story:** As a developer, I want to open a repository from the command line by passing its path as an argument, so that I can integrate gitv into my terminal workflow.

#### Acceptance Criteria

1. THE gitv_Application SHALL accept a repository path as a command-line argument (e.g., `gitv /path/to/repo`)
2. WHEN a valid repository path is provided as a CLI argument, THE gitv_Application SHALL open that repository directly, bypassing the welcome screen
3. WHEN an invalid or non-repository path is provided, THE gitv_Application SHALL display an error and fall back to the welcome screen
4. THE gitv_Application SHALL support opening multiple paths as separate windows (e.g., `gitv /repo1 /repo2` spawns two independent windows)

### Requirement 43: Uncommitted Changes Diff

**User Story:** As a developer, I want to view the diff of my staged and unstaged changes, so that I can review what I've modified before committing.

#### Acceptance Criteria

1. THE gitv_Application SHALL display a "Working Changes" entry at the top of the commit list (or as a virtual node in the graph)
2. WHEN the "Working Changes" entry is selected, THE gitv_Application SHALL display separate diffs for staged changes, unstaged changes, and the combined diff against HEAD
3. THE gitv_Application SHALL allow toggling between staged-only, unstaged-only, and combined diff views
4. THE gitv_Application SHALL update the working changes diff when filesystem changes are detected (per Req 22)

### Requirement 44: File Tree Browser

**User Story:** As a developer, I want to browse the full file tree of the repository at any commit, so that I can view and inspect any file's contents at any point in history.

#### Acceptance Criteria

1. THE gitv_Application SHALL display a file tree browser showing the full repository contents at the selected commit (or at HEAD when no commit is selected)
2. WHEN a file is selected in the tree, THE gitv_Application SHALL display its contents
3. THE file tree SHALL indicate file type with icons (or equivalent visual distinction)
4. THE file tree SHALL support expanding and collapsing directories
5. THE file tree SHALL update when a different commit is selected, reflecting the tree state at that commit

### Requirement 45: Multi-Instance Architecture

**User Story:** As a developer, I want gitv to open each repository in its own window, so that I can view multiple repos independently without state-sharing complexity.

#### Acceptance Criteria

1. THE gitv_Application SHALL NOT enforce single-instance behavior — each launch creates a new independent window
2. Each window SHALL manage its own state independently (commits, graph, selected commit, scroll, filters, sidebar)
3. Persistent data (preferences, recent repositories) SHALL be shared across instances via the filesystem
4. THE gitv_Application SHALL NOT implement inter-instance communication or tab management
5. THE gitv_Application SHALL support opening a repository in a new window via the native file dialog using Ctrl+Shift+O / Cmd+Shift+O or the command palette

### Requirement 46: Context Menu Actions

**User Story:** As a developer, I want to right-click on commits, files, and other elements to access common actions, so that I can work efficiently without memorizing shortcuts.

#### Acceptance Criteria

1. THE gitv_Application SHALL display a context menu when right-clicking a commit, offering: copy SHA, copy short SHA, copy commit message, compare with selected (if applicable), view file tree at this commit
2. THE gitv_Application SHALL display a context menu when right-clicking a file in the file tree, offering: copy file path, view file history (with --follow), view blame
3. THE gitv_Application SHALL display a context menu when right-clicking a branch, offering: copy branch name, filter to this branch, show first-parent only
4. THE gitv_Application SHALL support keyboard shortcut alternatives for all context menu actions

### Requirement 47: Drag-and-Drop

**User Story:** As a developer, I want to open a repository by dragging a folder onto the gitv window, so that I can open repos quickly from my file manager.

#### Acceptance Criteria

1. THE gitv_Application SHALL accept a folder dragged onto its window as a repository open request
2. WHEN a valid Git repository folder is dropped, THE gitv_Application SHALL open it in the current window, replacing the current view
3. WHEN a non-repository folder is dropped, THE gitv_Application SHALL display an error message

### Requirement 48: Commit Message Rendering

**User Story:** As a developer, I want commit messages to be displayed clearly with the summary line prominent and the body accessible, so that I can quickly scan history and read details when needed.

#### Acceptance Criteria

1. THE commit list SHALL display the commit summary (first line of the message) by default
2. THE commit detail panel SHALL display the full commit message including the body
3. THE commit message display SHALL wrap long lines appropriately for the available width
4. THE commit detail panel SHALL render common markdown formatting in commit messages (bold, italic, code spans, links, bullet lists)
5. THE commit list SHALL visually distinguish the summary line from the body when showing expanded commit details

### Requirement 49: Graph Color Accessibility

**User Story:** As a developer with color vision deficiency, I want the commit graph to use distinguishable colors and non-color indicators, so that I can follow branch lines accurately.

#### Acceptance Criteria

1. THE commit graph SHALL use a color palette designed for distinguishability under common color vision deficiencies (e.g., colorblind-safe palette)
2. THE commit graph SHALL use non-color indicators in addition to color to distinguish branches (e.g., line dash patterns, node shapes)
3. THE settings SHALL allow users to select from alternative color palettes (default, high contrast, deuteranopia-safe, protanopia-safe, tritanopia-safe)

### Requirement 50: Commit Count Display

**User Story:** As a developer, I want to see the total number of commits in the current view, so that I understand the scope of the history I'm browsing.

#### Acceptance Criteria

1. THE gitv_Application SHALL display the total commit count in the status bar when no filter is active
2. WHEN a filter is active, THE gitv_Application SHALL display both the matching commit count and the total commit count (e.g., "142 of 10,847 commits")
3. THE commit count SHALL update as streaming data loads (showing loaded count while loading)

### Requirement 51: Auto-Update

**User Story:** As a developer, I want gitv to notify me when a new version is available, so that I can stay up to date without manually checking.

#### Acceptance Criteria

1. THE gitv_Application SHALL check for updates on launch (asynchronously, without blocking the UI)
2. WHEN a new version is available, THE gitv_Application SHALL display a non-intrusive notification in the status bar
3. THE update check SHALL be disabled if the application has no network connectivity (per Req 40.3)
4. THE settings SHALL allow users to disable automatic update checks
5. THE update notification SHALL NOT auto-download or auto-install updates

---

> Requirements 52-58 added to close feature gaps with gitk (all read-only display/filtering features).

### Requirement 52: Color-by-Author Mode

**User Story:** As a developer, I want to color the commit graph by author instead of by branch, so that I can see at a glance who contributed what in multi-contributor repositories.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a "Color by Author" toggle in the View menu or toolbar
2. WHEN "Color by Author" mode is active, THE commit graph SHALL color nodes and edges by author identity instead of by branch
3. Author colors SHALL be assigned from the colorblind-safe palette (Req 49.1) and SHALL remain consistent within a session
4. THE gitv_Application SHALL display a color legend mapping authors to colors, accessible from the toolbar when the mode is active
5. THE default coloring mode SHALL be "by branch" (current behavior)
6. WHEN the author legend exceeds 20 entries, THE legend SHALL collapse with a scrollbar

### Requirement 53: Merge Commit Filtering

**User Story:** As a developer, I want to hide merge commits from the graph, so that I can focus on actual code changes in repositories with many merge commits.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a "Hide Merges" toggle in the toolbar or filter controls
2. WHEN "Hide Merges" is active, THE commit graph and commit list SHALL exclude merge commits (commits with more than one parent)
3. WHEN merge commits are hidden, THE graph edges SHALL reconnect through the hidden merge to its first parent, maintaining visual continuity
4. THE status bar SHALL indicate when merge filtering is active
5. THE "Hide Merges" filter SHALL combine with other active filters (branch filter, date range, etc.)

### Requirement 54: Diff View Options

**User Story:** As a developer, I want to choose how diffs are displayed (normal, word-level, stat-only) and control whitespace handling, so that I can review changes in the most effective way for the context.

#### Acceptance Criteria

1. THE diff viewer SHALL provide a mode dropdown with the following modes:
   - **Normal**: Standard unified diff with +/- line markers (default)
   - **Word diff**: Word-level changes highlighted inline (equivalent to `--word-diff=color`)
   - **Stat only**: File list with +/- line counts only, no diff content
2. THE diff viewer SHALL provide a whitespace modifier dropdown applicable to Normal and Word diff modes:
   - **None**: Show all whitespace changes (default)
   - **Ignore space change**: Ignore changes in amount of whitespace (`-w`)
   - **Ignore all space**: Ignore all whitespace
   - **Ignore blank lines**: Ignore changes whose lines are all blank
3. WHEN "Stat only" mode is active, THE whitespace modifier SHALL be hidden (not applicable)
4. THE diff mode and whitespace modifier SHALL persist for the current session but reset on restart
5. THE diff mode SHALL apply to all diff views (commit diff, comparison diff, stash diff, working changes diff)

### Requirement 55: CLI Revision Ranges

**User Story:** As a developer, I want to open gitv with a specific revision range from the command line, so that I can quickly view a focused slice of history (e.g., `gitv /repo v1.0..v2.0`).

#### Acceptance Criteria

1. THE gitv_Application CLI SHALL accept a revision range argument after the repository path (e.g., `gitv /path/to/repo HEAD~10..HEAD`)
2. THE CLI SHALL support the following revision range formats:
   - Single revision: `gitv /repo v2.0` (show history reachable from that rev)
   - Double-dot: `gitv /repo v1.0..v2.0` (commits in v2.0 not in v1.0)
   - Triple-dot: `gitv /repo main...feature` (symmetric difference)
3. THE CLI SHALL support additional filter flags: `--branches=<pattern>`, `--author=<pattern>`, `-- <file/path>`
4. WHEN a valid revision range is provided, THE gitv_Application SHALL open the repo with that range filter pre-applied (same behavior as Req 17)
5. WHEN an invalid revision or range is provided, THE gitv_Application SHALL display an error message and fall back to showing full history

### Requirement 56: Commit Highlighting and Dimming

**User Story:** As a developer, I want unselected commits to be visually dimmed so that the selected commit and related context stand out clearly in large graphs.

#### Acceptance Criteria

1. WHEN a commit is selected, THE gitv_Application SHALL dim all other commits in the graph and commit list (reduced opacity)
2. WHEN a file path filter is active (Req 6.5), THE gitv_Application SHALL dim commits that do NOT touch the filtered file while showing matching commits at full brightness
3. THE gitv_Application SHALL support toggling a persistent highlight on a commit (double-click or keyboard shortcut), visually distinct from selection
4. WHEN Escape is pressed, THE gitv_Application SHALL clear all dimming and return all commits to full brightness
5. THE dimming behavior SHALL be subtle enough to maintain readability — dimmed commits remain identifiable, not invisible

### Requirement 57: Graph Orientation

**User Story:** As a developer, I want to choose whether the commit graph flows top-to-bottom or bottom-to-top, so that I can match my preferred reading direction.

#### Acceptance Criteria

1. THE commit graph SHALL display in top-to-bottom orientation by default (newest commits at top)
2. THE gitv_Application SHALL provide a toggle in the View menu to switch to bottom-to-top orientation (newest commits at bottom)
3. WHEN orientation is toggled, THE graph SHALL re-render with the Y-axis flipped; scroll position SHALL map to the equivalent location
4. THE orientation preference SHALL persist across sessions
5. THE keyboard shortcut for toggling orientation SHALL be displayed in the View menu

### Requirement 58: Navigation by Author

**User Story:** As a developer, I want to jump to the next or previous commit by the same author, so that I can trace a specific contributor's work through the history.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide keyboard shortcuts to jump to the next commit by the same author as the currently selected commit
2. THE gitv_Application SHALL provide keyboard shortcuts to jump to the previous commit by the same author
3. WHEN author navigation reaches the end of the list, THE gitv_Application SHALL wrap around or indicate no more matches
4. THE keyboard shortcuts SHALL be listed in the keyboard shortcut help dialog (Req 13.4)

### Requirement 59: Panel Layout Persistence

**User Story:** As a developer who uses a tiling window manager, I want gitv to restore panel layouts sensibly even if the window was resized to an unusual geometry when I last closed it, so that I never end up with unusable column or pane widths.

#### Acceptance Criteria

1. THE gitv_Application SHALL persist resizable panel dimensions (sidebar width, detail panel height, graph/list column split) and window geometry (size, position) between sessions
2. THE gitv_Application SHALL define minimum usable dimensions for each panel: sidebar ≥ 150px wide, detail panel ≥ 200px tall, commit graph and commit list each ≥ 100px wide
3. THE gitv_Application SHALL define maximum fractions for each panel: sidebar ≤ 40% of window width, detail panel ≤ 50% of window height
4. WHEN persisted layout values are restored, THE gitv_Application SHALL clamp each value to its min/max bounds: `clamp(saved, min, window_dimension * max_fraction)`
5. WHEN the restored window size is smaller than the sum of all panel minimums, THE gitv_Application SHALL fall back to proportional default layout (e.g., sidebar 20%, graph 50%, list 30%)
6. THE gitv_Application SHALL clamp the restored window position to remain within the visible bounds of the current screen, preventing off-screen windows
7. ON first launch (no persisted layout), THE gitv_Application SHALL use proportional default layouts appropriate for the current window size
8. WHEN a panel is being resized via drag, THE gitv_Application SHALL debounce layout updates — applying at most one reflow per animation frame (≤ 16ms), and SHALL NOT trigger graph recalculation or full re-render during the drag gesture
9. THE synchronized scroller (Req 28) SHALL maintain graph/list alignment via CSS grid or flex layout, not by recalculating row positions on resize events
10. WHEN a panel resize drag ends, THE gitv_Application SHALL persist the final dimensions immediately; intermediate drag positions SHALL NOT be written to disk

### Requirement 60: Binary File Handling

**User Story:** As a developer, I want gitv to handle binary files gracefully in the diff viewer and file content viewer, so that I can see which binary files changed without errors or garbled output.

#### Acceptance Criteria

1. THE diff viewer SHALL detect binary files and display a "Binary file" placeholder message instead of attempting to render diff hunks
2. THE "Binary file" placeholder SHALL display the file path, change type (added/modified/deleted), and file size for both old and new versions
3. THE file content viewer SHALL display a "Binary file — cannot display" message when attempting to view a binary file
4. THE file tree SHALL indicate binary files with a distinct icon or visual marker
5. THE `FileDiff` data model SHALL include an `is_binary` flag
6. WHEN a commit touches both text and binary files, THE file list SHALL show both, with binary files displaying their placeholder in the diff panel when selected

### Requirement 61: Large Diff Lazy Loading

**User Story:** As a developer, I want gitv to handle diffs that touch many files or very large files without hanging, so that I can review any commit efficiently regardless of size.

#### Acceptance Criteria

1. WHEN a commit diff is requested, THE gitv_Application SHALL first load only the file list with stats (file paths, change types, line counts), without full hunk content
2. WHEN a file in the diff file list is selected, THE gitv_Application SHALL load that individual file's diff hunks on demand
3. THE gitv_Application SHALL apply a per-file line-count limit (default: 10,000 lines) for diff display, with a "Show full diff" affordance to expand beyond the limit
4. THE gitv_Application SHALL display a loading indicator while individual file diffs are being fetched
5. THE file list with stats SHALL load within 200ms regardless of total diff size
6. THE gitv_Application SHALL indicate the total number of changed files in the diff header (e.g., "47 files changed")

### Requirement 62: Concurrent Operation Handling

**User Story:** As a developer, I want gitv to handle concurrent operations gracefully (searching while streaming, filtering while loading), so that the application never shows inconsistent state or becomes unresponsive.

#### Acceptance Criteria

1. THE gitv_Application SHALL maintain an operation state per tab that tracks the current loading/filtering state: `Idle`, `StreamingCommits`, `Searching`, `ApplyingFilter`
2. WHEN a user applies a new filter while commit streaming is in progress, THE gitv_Application SHALL cancel the current stream and start a new stream with the updated filter
3. WHEN a user selects a commit while streaming is in progress, THE gitv_Application SHALL allow the selection and load the commit details concurrently without interrupting the stream
5. THE gitv_Application SHALL NOT queue multiple conflicting operations — only the most recent operation SHALL execute, prior conflicting operations SHALL be canceled
6. THE status bar SHALL indicate the current operation state (e.g., "Loading commits...", "Searching...", "Applying filter...")

### Requirement 63: Mid-Stream Error Recovery

**User Story:** As a developer, I want gitv to show me what it loaded so far even if a Git operation fails partway through, so that I can still work with partial results.

#### Acceptance Criteria

1. WHEN a commit stream fails mid-stream, THE gitv_Application SHALL retain and display all commits received before the failure
2. THE gitv_Application SHALL display a clear "Loading incomplete" indicator in the status bar when partial data is shown due to a stream error
3. THE gitv_Application SHALL provide a "Retry" action adjacent to the "Loading incomplete" indicator to restart the failed operation
4. WHEN a diff fetch fails for an individual file (Req 61), THE gitv_Application SHALL display an error message for that file without affecting other files in the diff list
5. THE gitv_Application SHALL log the error details for troubleshooting (per Req 15.4)

### Requirement 64: Submodule Support

**User Story:** As a developer working with repositories that contain submodules, I want to see submodule references in the file tree and understand when commits update submodule pointers, so that I can track submodule changes.

#### Acceptance Criteria

1. THE file tree SHALL display submodule entries with a distinct icon and the pinned commit short SHA (e.g., " submodule/ [abc1234]")
2. THE file tree SHALL indicate submodules as a distinct node type, separate from files, directories, and symlinks
3. WHEN a commit modifies a submodule pointer, THE diff file list SHALL display the change as a submodule update showing the old and new commit SHAs
4. THE diff viewer SHALL display submodule changes as "Submodule updated: old_sha → new_sha" instead of attempting a file-level diff
5. THE context menu for a submodule in the file tree SHALL offer "Copy submodule path" and "Copy pinned commit SHA"

### Requirement 65: Bare Repository Handling

**User Story:** As a developer working with bare repositories (e.g., mirror clones, server-side repos), I want gitv to open them and show commit history, so that I can browse any Git repository.

#### Acceptance Criteria

1. THE gitv_Application SHALL open bare repositories and display commit history, branches, tags, and the commit graph normally
2. WHEN a bare repository is opened, THE gitv_Application SHALL disable the "Working Changes" entry (Req 43) since there is no working tree
3. WHEN a bare repository is opened, THE gitv_Application SHALL disable the "Working Changes" entry — there is no working tree to watch if applicable
4. THE status bar SHALL indicate when a bare repository is open (e.g., "bare repository" label)
5. THE file tree browser (Req 44) SHALL display files at HEAD commit only — there is no working directory to browse
6. WHEN a non-bare repository's `.git` directory is opened directly, THE gitv_Application SHALL treat it as a bare repository

### Requirement 66: Notification System

**User Story:** As a developer, I want gitv to show transient, non-intrusive notifications for informational events, so that I know when background operations complete without being interrupted.

#### Acceptance Criteria

1. THE gitv_Application SHALL display notifications as transient toast messages that appear and auto-dismiss
2. THE notification system SHALL support three severity levels: info (auto-dismiss after 3s), warning (auto-dismiss after 5s, amber color), error (manual dismiss only, red color)
3. WHEN multiple notifications are active simultaneously, THE gitv_Application SHALL stack them vertically without overlapping
4. THE gitv_Application SHALL use notifications for: refresh completion events (Req 22.2), clipboard copy confirmations, cache load timing, search completion with match count, and mid-stream errors (Req 63)
5. THE notification area SHALL be positioned in the bottom-right corner of the window, above the status bar
6. THE notification component SHALL respect reduced-motion preferences (Req 67) — using instant appearance instead of slide-in animations

### Requirement 67: Accessibility — Reduced Motion and Focus Management

**User Story:** As a developer with accessibility needs, I want gitv to respect reduced-motion preferences and manage focus properly during navigation, so that I can use the application comfortably.

#### Acceptance Criteria

1. THE gitv_Application SHALL detect the system `prefers-reduced-motion` setting and disable non-essential animations (graph transitions, scroll easing, panel slide animations) when active
2. WHEN `prefers-reduced-motion` is active, THE gitv_Application SHALL use instant transitions instead of animated ones for all state changes
3. WHEN a modal dialog opens, THE focus SHALL be trapped within the modal; WHEN the modal closes, THE focus SHALL return to the element that opened it
4. WHEN a programmatic navigation occurs (e.g., clicking a reflog entry navigates to a commit, clicking a stash entry scrolls the graph), THE focus SHALL move to the target element
5. THE gitv_Application SHALL provide an `aria-live` region for announcing dynamic content changes (commit count updates, search results, refresh events) to screen readers
6. WHEN a tab is switched, THE focus SHALL move to the commit list in the new tab
7. WHEN the command palette opens, THE focus SHALL move to the search input; WHEN it closes, THE focus SHALL return to the previously focused element

### Requirement 68: Structured Tracing and Logging

**User Story:** As a gitv developer, I want structured, level-filtered logging throughout the application, so that I can diagnose issues efficiently in development and production.

#### Acceptance Criteria

1. THE gitv_Application SHALL use the `tracing` crate for all Rust-side logging with structured spans (command name, repo path, duration)
2. THE gitv_Application SHALL support configurable log levels: error, warn, info, debug, trace — with a default of `info` in release builds and `debug` in debug builds
3. THE gitv_Application SHALL allow users to change the log level via CLI flag (e.g., `gitv --log-level=debug`) or environment variable (`GITV_LOG=debug`)
4. THE gitv_Application SHALL write logs to a rotating file in the platform-appropriate log directory, with a maximum file size of 10MB and at most 3 rotated files
5. EACH Tauri IPC command invocation SHALL be traced with its name, input parameters (excluding repo path for brevity), duration, and result (ok/error)
6. THE frontend SHALL forward `console.warn` and `console.error` to the Rust logging subsystem so that all logs are in one place
7. THE settings modal SHALL display the log file path so users can locate it for bug reports

### Requirement 69: Debug and Performance Overlay

**User Story:** As a gitv developer or power user, I want a toggleable debug overlay that shows real-time performance metrics, so that I can identify performance issues and verify targets are being met.

#### Acceptance Criteria

1. THE gitv_Application SHALL provide a debug overlay toggled via keyboard shortcut (F12 or Ctrl+Shift+D / Cmd+Shift+D)
2. WHEN the debug overlay is visible, IT SHALL display: current FPS, memory usage (RSS), loaded commit count, streaming progress, current operation state (Req 62)
3. THE debug overlay SHALL display the last 10 IPC command durations (command name, duration in ms)
4. THE debug overlay SHALL display GPU stats: draw call count, vertex count, viewport size
5. THE debug overlay SHALL display cache stats: cache hit/miss ratio, last cache load duration
6. THE debug overlay SHALL be positioned in the top-right corner, semi-transparent, and SHALL NOT interfere with normal UI interaction
7. THE debug overlay SHALL NOT be available in release builds by default, but SHALL be enabled via CLI flag (`gitv --debug-overlay`) for power-user diagnostics

### Requirement 70: Crash Diagnostics

**User Story:** As a gitv developer, I want the application to capture diagnostic information on crash, so that I can reproduce and fix bugs reported by users.

#### Acceptance Criteria

1. THE gitv_Application SHALL install a panic handler that captures: panic message, source location, backtrace (when available), and application version
2. THE gitv_Application SHALL install a frontend error handler that captures: error message, stack trace, browser/webview version, and current route/component
3. WHEN a crash occurs, THE gitv_Application SHALL write the diagnostic information to a crash log file in the platform-appropriate log directory, separate from the rotating log file
4. THE gitv_Application SHALL retain at most 5 crash log files, evicting the oldest
5. THE settings modal SHALL display a button to open the log directory, making it easy for users to attach logs to bug reports
6. THE gitv_Application SHALL NOT include any personally identifiable information or repository content in crash logs (only metadata: paths, SHAs, error messages, stack traces)
