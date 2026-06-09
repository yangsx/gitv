# gitv

A modern, cross-platform Git visualization tool built with Rust and Tauri. A contemporary reimplementation of gitk with GPU-accelerated rendering, streaming data loading, and a polished UI.

## Features

- **GPU-accelerated commit graph** — wgpu-rendered with Canvas 2D fallback; virtualized viewport for 100k+ commits
- **Branch/author coloring** — color by branch or by author, with colorblind-safe palettes
- **Stash visualization** — stash nodes as gold diamonds on parent commit rows, with combined or split diffs
- **Diff viewer** — unified or side-by-side, normal/word-diff/stat-only modes, whitespace modifiers
- **File tree browser** — browse repository contents at any commit, view file contents and blame
- **Reflog** — browse reflog entries, navigate to any historical commit
- **Search** — RoaringBitmap inverted index for sub-100ms commit search on large repos
- **Persistent cache** — disk cache (postcard) for instant re-open of previously visited repos
- **Command palette** — fuzzy search for commands, recent repos, and commit navigation
- **Keyboard navigation** — arrow keys, j/k, Page Up/Down, Home/End, context menus
- **Preferences** — persistent JSON config at `$XDG_CONFIG_HOME/gitv/preferences.json`
- **Multi-instance** — each launch opens an independent window; no tab complexity
- **CLI** — `gitv /repo`, revision ranges (`v1.0..v2.0`), filter flags (`--branches`, `--author`)
- **i18n** — English and Simplified Chinese

## Tech Stack

| Layer | Tool |
|-------|------|
| App framework | Tauri 2.0 |
| Git library | gix (gitoxide) |
| GPU rendering | wgpu |
| Frontend | Svelte 5, SvelteKit (static adapter), TypeScript, Tailwind CSS |
| Build | Vite, cargo |
| Binary serialization | postcard |
| Search index | RoaringBitmap |
| Logging | tracing + tracing-subscriber + tracing-appender |
| Test runner | cargo-nextest |

## Project Structure

```
gitv/
├── src-tauri/                # Rust backend — Tauri commands, wgpu state
│   ├── src/
│   │   ├── commands/         # IPC commands (repo, graph, diff, render, etc.)
│   │   └── wgpu_state.rs    # Lazy GPU device holder
│   └── Cargo.toml
├── crates/
│   ├── gitv-git-core/        # Pure Rust Git logic crate (no Tauri deps)
│   │   ├── src/
│   │   │   ├── repository.rs # gix-based repo abstraction
│   │   │   ├── graph/        # Graph calculator, layout, stash insertion
│   │   │   ├── search.rs     # RoaringBitmap search engine
│   │   │   ├── stream.rs     # Streaming commit iterator
│   │   │   ├── cache.rs      # Persistent disk cache
│   │   │   ├── watcher.rs    # File watcher
│   │   │   └── models.rs     # Core data types (Oid, CommitInfo, etc.)
│   │   └── tests/            # 83 passing tests
│   └── gitv-wgpu-renderer/   # Offscreen wgpu renderer
│       ├── src/
│       │   ├── lib.rs        # WgpuRenderer (init, render, readback)
│       │   ├── renderer.rs   # Render pipeline, staging buffer, timing
│       │   ├── shaders.rs    # WGSL shaders (node SDF, edge dash/dot)
│       │   └── vertex.rs     # Vertex types (NodeInstance, EdgeVertex)
│       └── Cargo.toml
├── frontend/                 # SvelteKit frontend
│   ├── src/
│   │   ├── routes/+page.svelte    # Main page (welcome / repo view)
│   │   ├── lib/
│   │   │   ├── components/        # UI components
│   │   │   │   ├── graph/         # WgpuGraph, GraphRenderer, graph-math.ts
│   │   │   │   ├── Sidebar/       # RefList, StashList, ReflogPanel
│   │   │   │   └── ...            # CommitList, DiffViewer, etc.
│   │   │   ├── stores/            # Svelte stores (prefs, repo, debug)
│   │   │   ├── bindings/types.ts  # IPC type definitions
│   │   │   └── locales/           # en.json, zh-CN.json
│   │   └── app.css               # Global styles + scrollbar customization
│   ├── package.json
│   └── svelte.config.js
├── design.md                 # Architecture document
├── requirements.md           # 70 requirements
└── AGENTS.md                 # AI agent instructions
```

## Getting Started

### Prerequisites

- Rust (latest stable, edition 2024)
- Node.js 20+ and npm
- Platform-specific: GTK 3+ development libraries (Linux)

### Build and Run

```bash
# Install frontend dependencies
cd frontend && npm install && cd ..

# Development mode (frontend + Tauri)
npm run dev

# Production build
npm run build
```

### Tauri CLI

```bash
# Development
cargo tauri dev

# Build release
cargo tauri build
```

## Usage

```bash
# Open a repository
gitv /path/to/repo

# Open with revision range
gitv /path/to/repo v1.0..v2.0

# Filter by branch or author
gitv /path/to/repo --branches=main --author=alice

# Open in a new window (multi-instance)
gitv /path/to/repo1 &
gitv /path/to/repo2 &
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+O` | Open repository |
| `Ctrl+P` | Command palette |
| `Ctrl+F` | Search |
| `Ctrl+R` | Refresh |
| `Ctrl+Shift+O` | Open in new window |
| `Arrow Up/Down`, `j/k` | Navigate commits |
| `Page Up/Down` | Jump pages |
| `Home/End` | Jump to first/last commit |
| `Escape` | Clear selection / close modal |
| `F12` | Debug overlay |

## Architecture

- **Decoupled Git core** (`gitv-git-core`) — pure Rust crate, no Tauri dependencies, 83 tests
- **GPU rendering** — wgpu offscreen pipeline, RGBA readback via binary IPC (bypasses JSON)
- **Binary IPC** — postcard serialization for commit batches (3-5x smaller, 5-10x faster than JSON)
- **Virtual scroll** — only visible commits rendered; graph and list scroll in sync
- **Persistent cache** — `$XDG_DATA_DIR/gitv/cache/`, postcard-serialized, ref-snapshot invalidation

## Development

### Code Quality Gates

```bash
# Rust
cargo fmt --check
cargo clippy -- -D warnings
cargo nextest run        # 83 tests
cargo doc --no-deps

# Frontend
cd frontend
npm run lint             # ESLint + Prettier
npm run check            # svelte-check (TypeScript)
npm run build            # Vite production build
```

### Test Coverage

```bash
# gitv-git-core crate
cargo tarpaulin --manifest-path crates/gitv-git-core/Cargo.toml
# Target: >= 80% line coverage
```

## License

MIT
