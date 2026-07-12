# gitv

[**简体中文版**](README.zh-CN.md)

A modern, cross-platform Git repository visualizer. GPU-accelerated commit graph,
streaming data, and persistent cache — built with Rust + Tauri.

Think gitk, but with Canvas 2D rendering, stash-as-graph-node display, sub-100ms
search, and instant re-open for cached repos.

## Features

- **Canvas 2D rendered commit graph** — virtualized viewport for 100k+ commits
  at 60 FPS
- **Color modes** — color graph by branch or by author; colorblind-safe palettes
  (deuteranopia, protanopia, tritanopia, high contrast)
- **Edge styles** — solid/dashed/dotted lines as non-color branch indicators
- **Stash browsing** — stashes appear as proper graph nodes with their own row
  and a branch-out edge, not gitk's two-node double-diff display
- **Diff viewer** — unified or side-by-side; normal, word-diff, and stat-only
  modes; whitespace modifiers (ignore space change, ignore all space, ignore
  blank lines); scroll-synced file list
- **Two-commit comparison** — Ctrl+Click or right-click to select any pair of
  commits and see the combined diff
- **Patch text search** — search commit diffs with regex, matching lines
  highlighted in-place within the diff viewer
- **Commit search** — RoaringBitmap inverted index for sub-100ms commit message
  and author search on 100k+ commit repos
- **File tree browser** — browse repository contents at any commit; view file
  contents, blame, and file history with rename following (`--follow`)
- **Reflog** — browse reflog entries, navigate to any commit (even dangling
  ones after rebase or reset)
- **Command palette** — Ctrl+P fuzzy search for all actions, recent repos,
  and navigation
- **Persistent cache** — disk cache (postcard serialized) for instant re-open
  of previously visited repos; incremental updates on ref changes
- **Keyboard navigation** — full keyboard control: arrow keys, J/K, Page Up/Down,
  Home/End, author jump, branch cycling, font zoom (Ctrl+= / Ctrl+- / Ctrl+0)
- **Multi-instance** — each launch opens an independent window; no tab
  complexity, no shared state
- **Preferences** — persistent JSON at `$XDG_CONFIG_HOME/gitv/preferences.json`
  with debounced auto-save; theme (dark/light/auto), font size, graph/diff
  defaults
- **i18n** — English, Simplified Chinese, German (community-contributed;
  drop a JSON in `locales/` to add a language)
- **CLI** — `gitv /path/to/repo`, `gitv /repo1 /repo2`
- **Debug overlay** — F12 toggles real-time FPS, memory, graph stats, IPC
  timing, and load phase timings

## Prerequisites

- **Rust** — latest stable (edition 2024)
- **Node.js** — 20+ and npm
- **Linux** — GTK 3+ development libraries (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`,
  `libxdo-dev`, etc. — see [Tauri docs](https://v2.tauri.app/start/prerequisites/))

## Quick Start

```bash
# Clone
git clone https://github.com/yangsx/gitv && cd gitv

# Install frontend deps
cd frontend && npm install && cd ..

# Run in development mode
cargo tauri dev

# Production build
cargo tauri build
```

The bundled packages will be in `target/release/bundle/`.

## CLI Usage

```bash
# Open a repository
gitv /path/to/repo

# Open multiple repositories (each in its own window)
gitv /repo1 /repo2

# Set log level (debug, trace)
gitv /path/to/repo --log-level=debug
```

`gitv --help`:

```
Modern Git repository visualizer

Usage: gitv [OPTIONS] [REPO_PATHS]...

Arguments:
  [REPO_PATHS]...

Options:
      --log-level <LOG_LEVEL>
      --debug-overlay
      --self-test <PATH>        Run headless graph self-test (human-readable summary to stderr)
      --self-test-json <PATH>   Run headless graph self-test (JSON output to stdout)
      --dump-graph <PATH>       Dump computed graph layout as text (nodes, edges, expanded paths)
      --dump-graph-json <PATH>  Dump computed graph layout as JSON (full layout + commits + diagnostics)
      --max-commits <N>         Maximum commits to process in self-test/dump mode (default: no limit)
      --hide-merges             Hide merge commits in dump output (applies to --dump-graph / --dump-graph-json only)
      --orientation <DIR>       Graph orientation in dump output: top-to-bottom or bottom-to-top (applies to --dump-graph / --dump-graph-json only)
  -h, --help                    Print help
  -V, --version                 Print version
```

### Headless Graph Analysis

gitv includes CLI commands for algorithm validation and debugging —
no GUI window is opened.

**Self-test** — validates graph layout integrity on a real repo:

```bash
gitv --self-test /path/to/repo
gitv --self-test-json /path/to/repo
gitv --self-test /path/to/repo --max-commits 1000
```

Sample output:

```
self-test: 45.5ms (compute 5.8ms), 225 nodes, 226 edges, 2 cols, 0 errors
  diagnostics: waste=0, waypoints=4, arrow_gaps=0, max_threads=2
  topology: 2 merges, longest_chain=216, fork_points=3
  hide_merges: 224 nodes, 225 edges, 0 errors
```

Exit code is 0 on success, 1 if layout errors are found. JSON variant
(`--self-test-json`) outputs to stdout for CI/automation.

**Dump graph** — exports the full computed layout for external analysis:

```bash
gitv --dump-graph /path/to/repo
gitv --dump-graph-json /path/to/repo | python3 -m json.tool | head -50

# With graph options
gitv --dump-graph /path/to/repo --hide-merges
gitv --dump-graph /path/to/repo --orientation bottom-to-top
```

Text output shows nodes (row, col, oid, message), edges (type, style,
waypoints, expanded path cells), diagnostics, and property checks.

JSON output includes:

- `nodes` — row, column, oid, message, author, parents, refs
- `edges` — from/to positions, type, style, waypoints, `arrow_gap`,
  `expanded_path` (every `(row, col)` cell traversed per segment)
- `diagnostics` — column waste, waypoints, arrow gaps, max threads
- `property_checks` — 7 checks with violation counts and samples
- `errors` — layout verification errors (empty if valid)

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **File** | |
| `Ctrl+O` | Open repository |
| `Ctrl+Shift+O` | Open repository in new window |
| `Ctrl+W` | Close repository (back to welcome) |
| `Ctrl+Q` | Quit application |
| `Ctrl+R` | Refresh |
| **Navigation** | |
| `↓` / `J` | Next commit |
| `↑` / `K` | Previous commit |
| `PageDown` | Next page |
| `PageUp` | Previous page |
| `Home` | First commit |
| `End` | Last commit |
| `Alt+N` | Next commit by same author |
| `Alt+P` | Previous commit by same author |
| **Graph** | |
| `Ctrl+Shift+M` | Toggle hide merges |
| `Ctrl+Shift+A` | Toggle color-by-author |
| `Ctrl+Shift+G` | Toggle graph orientation |
| **View** | |
| `Ctrl+M` | Toggle fullscreen |
| `Ctrl+,` | Preferences |
| `F12` / `Ctrl+Shift+D` | Debug overlay |
| **Branch** | |
| `Alt+B` | Next branch (focus) |
| `Alt+Shift+B` | Previous branch (focus) |
| **Help** | |
| `Ctrl+P` | Command palette |
| `F1` / `Ctrl+/` | Keyboard shortcuts help |
| `Escape` | Clear selection / close modal / exit fullscreen |
| `Ctrl+Click` / `Cmd+Click` | Select second commit for comparison |

## Why gitv instead of gitk?

| Feature | gitv | gitk |
|---------|------|------|
| Rendering | Canvas 2D (GPU-accelerated) | Tk canvas (CPU) |
| Stash display | Single graph node with branch-out edge + combined diff | Two-node double-diff display |
| Search | RoaringBitmap indexed (sub-100ms on 100k commits) | Linear scan |
| Diff modes | Normal, word-diff, stat-only; whitespace modifiers | Normal only |
| Open model | Independent launcher with repository picker | Must be run from inside a repo |
| Multi-repo | Multi-instance, each in its own window | Single-window tabs |
| Cache | Persistent disk cache for instant re-open | None |
| Color modes | By-branch, by-author, colorblind-safe palettes | By-branch only |
| Reflog | Dedicated sidebar panel with entry navigation | Not directly supported |
| Preference persistence | Auto-saved JSON config | Session-only |

## Architecture

```
gitv/
├── src-tauri/                  # Rust backend (Tauri 2.0)
│   ├── src/
│   │   ├── commands/           # IPC commands
│   │   └── lib.rs              # App setup, state, command registration
│   └── Cargo.toml
├── crates/
│   ├── gitv-git-core/          # Pure Rust Git logic (no Tauri deps) — 170 tests
│   │   └── src/
│   │       ├── repository.rs   # gix-based repo abstraction
│   │       ├── graph/          # Layout calculator, stash insertion
│   │       ├── search/         # RoaringBitmap search engine
│   │       ├── stream/         # Streaming commit iterator
│   │       ├── cache/          # Persistent disk cache
│   │       └── models.rs       # Core types (Oid, CommitInfo, Diff, etc.)
├── frontend/                   # Svelte 5 + TypeScript
│   ├── src/
│   │   ├── routes/             # +layout.svelte, +page.svelte
│   │   └── lib/
│   │       ├── components/     # 20+ components (flat, no deep nesting)
│   │       ├── stores/         # 8 stores (repository, preferences, layout, etc.)
│   │       ├── graph/          # graph-math.ts, edge-interaction.ts
│   │       ├── locales/        # en.json, zh-CN.json, de.json
│   │       └── utils/          # a11y.ts, markdown.ts, format-date.ts
│   └── package.json
├── design.md                   # Architecture document (3196 lines)
├── requirements.md             # 70 requirements
└── AGENTS.md                   # AI agent conventions and toolchain
```

### Key Design Decisions

- **Decoupled Git core**: `gitv-git-core` is a standalone crate with trait-based
  interfaces for mocking — independently testable, no Tauri dependency
- **Oid**: 20-byte binary newtype (`[u8; 20]`), not a String — 3x less memory,
  faster hashing
- **Binary IPC**: postcard serialization for commit batches (3-5x smaller,
  5-10x faster than JSON)
- **Virtual scroll**: only visible commits rendered; graph canvas and commit
  list share a synchronized scroll container

## Troubleshooting

| Problem | Likely cause | Fix |
|---------|-------------|-----|
| Large repo feels slow on first open | No cache yet — full traversal required | Normal; second open will be < 200ms |
| "Not a Git repository" on valid path | Subdirectory of a repo? | gitv discovers the root — try the repo root |
| Cache is stale or wrong | Remote pushed new commits | Click the refresh button (Ctrl+R) |
| Keyboard shortcuts don't work | Focus is in an input field | Ctrl shortcuts still work; plain key shortcuts (J, K) require focus outside inputs |

## Project Documentation

- [`design.md`](design.md) — Full architecture document: component hierarchy,
  data models, CLI, keyboard shortcuts, ADRs, testing strategy, accessibility
- [`requirements.md`](requirements.md) — 70 requirements with acceptance criteria
- [`AGENTS.md`](AGENTS.md) — AI agent conventions, code quality gates, toolchain

## Development

### One-time Setup

```bash
# Rust toolchain (if not already installed)
rustup default stable

# Frontend dependencies
cd frontend && npm install && cd ..

# Linux: system deps (see Tauri prerequisites for your distro)
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libxdo-dev \
  libappindicator3-dev librsvg2-dev patchelf libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev

# Coverage tooling (optional, for running cargo llvm-cov)
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

### Dev Workflow

Run the app in development mode with hot-reload:

```bash
cargo tauri dev
```

This starts a Vite dev server for the frontend (HMR on file changes) and
compiles the Rust backend on first launch. The Tauri window opens automatically.

**Tauri DevTools**: right-click the window → Inspect Element to open the
WebView DevTools (console, network, elements).

### Project Layout

See [Architecture](#architecture) above. Key directories:

| Directory | Purpose |
|-----------|---------|
| `src-tauri/` | Rust backend (Tauri commands) |
| `crates/gitv-git-core/` | Pure-Rust Git logic (no Tauri deps) |
| `frontend/src/lib/components/` | Svelte 5 components (~20, flat) |
| `frontend/src/lib/stores/` | Svelte stores (8 files) |
| `frontend/src/lib/locales/` | i18n JSON files |
| `.github/workflows/` | CI workflows |

### Tests

```bash
# Run all Rust tests (nextest recommended)
cargo nextest run --workspace

# Run a specific crate
cargo nextest run -p gitv-git-core

# Run a specific test
cargo nextest run -p gitv-git-core --test graph_tests

# Run with cargo test (fallback)
cargo test --workspace
```

170 tests in `gitv-git-core` — no Tauri dependency, fully mockable.

### Benchmarks

```bash
# Run all criterion benchmarks
cargo bench --manifest-path crates/gitv-git-core/Cargo.toml

# Run a specific benchmark
cargo bench --bench graph_bench
```

CI compares PR results against the main branch baseline and fails on
>10% regression.

### Performance Targets

Benchmark results are checked against absolute budgets in CI via
`scripts/check_bench_targets.py` (run locally with `python3
scripts/check_bench_targets.py target/criterion`). Current measurements on an
Intel Core i7-1165G7:

| Status | Benchmark | Measured | Budget | vs Budget |
|--------|-----------|----------|--------|-----------|
| ✅ PASS | Search (text, 100k commits) < 100 ms | 5.796 ms | 100.000 ms | 94.2% under budget |
| ✅ PASS | Search (regex, 100k commits) < 100 ms | 13.825 ms | 100.000 ms | 86.2% under budget |
| ✅ PASS | Search (author, 100k commits) < 100 ms | 933.606 µs | 100.000 ms | 99.1% under budget |
| ✅ PASS | Search index build (100k commits) < 5 s | 147.937 ms | 5.000 s | 97.0% under budget |
| ✅ PASS | Graph layout linear (10k commits) < 2 s | 19.146 ms | 2.000 s | 99.0% under budget |
| ✅ PASS | Graph layout branchy (10k commits) < 2 s | 19.949 ms | 2.000 s | 99.0% under budget |

### Coverage

```bash
cargo llvm-cov -p gitv-git-core --html --output-dir coverage-report \
  --ignore-filename-regex '<WORKSPACE>'
# Target: >= 80% line coverage
```

### Debugging

```bash
# Launch with debug overlay
gitv /path/to/repo --debug-overlay

# Enable verbose logging
gitv /path/to/repo --log-level=trace

# Combined
gitv /path/to/repo --debug-overlay --log-level=debug
```

- **F12** toggles the debug overlay (FPS, memory, graph stats, IPC timing)
- Logs are written to rolling files in the app data directory
- Panic captures backtrace automatically — crash logs retained for diagnosis

### Adding a Language (i18n)

1. Create `frontend/src/lib/locales/xx.json` (lowercase stem, e.g. `ja.json`)
2. Translate the strings from `en.json`
3. The locale is auto-discovered by `import.meta.glob` — no registration needed

Existing: English (`en.json`), Simplified Chinese (`zh-CN.json`), German (`de.json`).

### CI Checklist

Before pushing, run all quality gates locally:

```bash
# Rust
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo nextest run --workspace
cargo doc --workspace --no-deps

# Frontend
cd frontend && npm run lint && npm run check && npm run build
```

The same checks run in CI (`.github/workflows/ci.yml`). Additional
workflows run coverage, security audit (weekly), and benchmarks on
push/PR.

### Making Changes

1. Create a feature branch from `main`
2. Make changes, ensuring all CI gates pass locally
3. Write or update tests for new functionality
4. Open a pull request
5. AI-generated commits include `Co-Authored-By` trailer

## License

MIT
