# Agent Instructions

## Project Status
**In development.** Core application has substantial source code in place:
- **Backend** (`src-tauri/`): Tauri commands for preferences, graph layout, commits, diff, repo operations, saved searches, file watching, diagnostics
- **Git core** (`crates/gitv-git-core/`): 99 tests passing — repository abstraction, graph calculator/layout, search engine (RoaringBitmap), streaming, file watching, disk cache, models
- **GPU renderer** (`crates/gitv-wgpu-renderer/`): wgpu-based GPU graph renderer with WGSL shaders, Canvas 2D fallback, and user preference toggle
- **Frontend** (`frontend/`): Svelte 5 + TypeScript with CommitGraph (canvas-based), CommitList, CommitDetailPanel (diff/whitespace controls), PreferencesModal (draggable), InfoDialog (draggable — shortcuts, logging, app info), Toolbar, SearchBar, Sidebar, FileTree, BlamePanel, CommandPalette, ContextMenu, DebugOverlay
- **Preferences**: Persistent JSON at `$XDG_CONFIG_HOME/gitv/preferences.json` with debounced auto-save, applies to graph/diff/view behavior
- Architecture design in `design.md`; full requirements in `requirements.md`

## Architecture
- **Rust + Tauri 2.0** desktop app: Git visualization tool (modern gitk)
- **Backend**: `src-tauri/` — Rust with `gix` (gitoxide) for Git ops, `wgpu` for GPU graph rendering
- **Decoupled Git core**: `crates/gitv-git-core/` — pure Rust, no Tauri deps, independently testable
- **Multi-instance**: each launch opens an independent window; no tab support
- **Frontend**: `frontend/` — SvelteKit + Svelte 5 + TypeScript + Tailwind + Vite
- **Pure Rust preferred**: use pure-Rust crates over C bindings; shell out to `git` CLI only as documented fallback

## Planned Project Layout
```
src-tauri/            # Rust backend (Tauri commands in src/commands/)
crates/gitv-git-core/ # Git logic crate (repository, graph, search, stream, watcher, cache, models)
crates/gitv-wgpu-renderer/ # GPU graph rendering (wgpu + WGSL shaders)
frontend/             # SvelteKit frontend (src/routes/, src/lib/components/, src/lib/stores/, src/lib/actions/, src/lib/bindings/, src/lib/graph/)
tests/                # Integration tests + fixtures
benches/              # (in crates/gitv-git-core/benches/)
```

## Key Design Decisions
- Git backend is a separate crate (`gitv-git-core`) with trait-based interfaces for mocking
- `Oid` is a 20-byte binary newtype (`[u8; 20]`), not a String — 3x less memory, faster hashing
- Commit graph uses GPU-accelerated rendering via wgpu with virtualized viewport
- Commits are streamed in batches with binary serialization (postcard) to avoid JSON IPC overhead
- Persistent disk cache for graph layout + metadata — re-open cached repos in <200ms
- Each window holds isolated repository state (own connection, filters, scroll position)
- Branch filtering happens at Git traversal layer, not UI layer
- SvelteKit with static adapter — single-window desktop app uses `+page.svelte`/`+layout.svelte` conventions; static adapter produces a SPA for Tauri
- Stashes displayed as proper graph nodes with their own row and a branch-out edge to the parent commit (not gitk's two-node double-diff display); combined diff by default with optional staged/unstaged split toggle (Req 38)
- Diff viewer supports normal, word-diff, and stat-only modes with whitespace modifiers (Req 54)
- Graph supports color-by-author mode (Req 52), merge filtering (Req 53), commit dimming (Req 56), and orientation toggle (Req 57)
- Edge interaction: clickable graph edges with cubic bezier hit-testing, hover highlighting (thicker same-curve + endpoint rings), and click-to-navigate (commit ef495f9)
- Edge styles (Solid/Dashed/Dotted) provide non-color branch indicators for colorblind accessibility, rendered via WGSL shaders on wgpu or Canvas 2D draw calls
- CLI accepts repo path arguments (`gitv /repo1 /repo2`) and `--log-level` flag (Req 42, Req 55 revision ranges not yet implemented)
- CLI argument parsing via `clap`
- Panel widths/heights are clamped on restore to min/max bounds — prevents unusable layouts from tiling WMs (Req 59)
- Structured tracing via `tracing` crate with rolling file logs; debug overlay with FPS/memory/IPC timing (Reqs 68-69)
- Crash diagnostics captured via panic hook with backtrace, retained as crash logs (Req 70)
- Preferences persisted as JSON at `$XDG_CONFIG_HOME/gitv/preferences.json` with atomic writes; debounced frontend auto-save (300ms) avoids excessive I/O
- Diff/whitespace controls in CommitDetailPanel use local `$state` overrides (not global preference saves); PreferencesModal sets global defaults
- Preferences dialog is draggable (no backdrop) to allow flexible placement alongside the graph
- Graph toolbar simplified — toggle buttons (color mode, hide merges, orientation) moved into Preferences dialog; gear and info (ℹ) icons remain on toolbar
- Dual renderer: wgpu GPU (WGSL shaders) with Canvas 2D fallback, user-selectable via `renderer` preference
- Commit messages: XSS-sanitized Markdown rendering with raw/markdown toggle (Req 48.4)
- Light theme and high contrast mode supported alongside default dark theme (Req 11.1, Req 14.3)
- Side-by-side diff view mode persisted as a preference (Req 54.5)
- Branch/tag/remote click focuses the graph on that ref via `focus_branch_oid`, dimming unrelated commits (Req 3.3, Req 32)
- Merged branch detection via HEAD ancestor set, displayed as `is_merged` on `BranchRef` in sidebar (Req 3.6)
- Locale support: English and Simplified Chinese with auto-detect on first launch (Req 21); extensible via `import.meta.glob` discovery — drop a new JSON in `locales/` to add a language; Rust `Language` enum with `Custom(String)` fallback for forward compatibility; locale file naming uses lowercase stems (`de.json`, `ja.json`) — existing `zh-CN.json` grandfathered
- Merged initial IPC: `getInitialData` combines repo info, commits, graph layout, refs, working changes, and timing in a single call (commit d4fe74a)
- Parallelized diff loading with 4 concurrent workers for faster commit detail display
- Repo switch: all selection state is cleared and CommitList is force-remounted via `{#key}` to prevent stale internal state from the previous repo
- Debug overlay available in all builds via F12/Ctrl+Shift+D — no CLI flag or feature gate required (commit 996dfc2)
- Font size zoom via Ctrl+=/Ctrl+-/Ctrl+0 (commit 746e15f)
- wgpu hover performance: base render cached as `ImageData`, hover/selection changes use fast Canvas 2D re-blit path (no IPC/GPU roundtrip) (commit a71db87)

## Tech Stack
| Layer | Tool |
|-------|------|
| App framework | Tauri 2.0 |
| Git library | gix (gitoxide) |
| GPU rendering | wgpu |
| Refresh | Manual (toolbar button) |
| Frontend | SvelteKit, Svelte 5, TypeScript, Svelte stores |
| Virtual list | svelte-virtual-scroll-list |
| Styling | Tailwind CSS |
| Build (frontend) | Vite |
| Batch serialization | postcard |
| Search index | RoaringBitmap inverted index |
| Logging | tracing + tracing-subscriber + tracing-appender |
| Test runner (Rust) | cargo-nextest |
| Benchmarks | criterion |
| Coverage | cargo-tarpaulin |

## Performance Targets
| Metric | Target |
|--------|--------|
| Cold start to welcome | < 500ms |
| First open (no cache, 100k commits) | < 5s |
| Re-open (cached, 100k commits) | < 200ms |
| Scroll FPS | 60 |
| Search (indexed, 100k commits) | < 100ms |
| Binary size | < 15MB |
| Working changes diff | < 50ms |

## Code Quality Gates

Before every commit checkpoint (phase milestones, PRs, substantive changes), ALL of the following MUST pass:

### Rust
- `cargo fmt --check` — zero formatting diffs
- `cargo clippy --workspace --all-targets -- -D warnings` — zero warnings, zero errors
- `cargo check` / `cargo build` — compiles cleanly
- `cargo doc --no-deps` — builds without warnings (public API documented)
- `cargo test` (or `cargo nextest run`) — all tests pass
- No `unwrap()` in non-test code — use `?`, `ok_or`, or explicit error handling
- No `todo!()` or `unimplemented!()` in committed code without a tracking issue

### Frontend (SvelteKit / TypeScript)
- `npm run lint` (ESLint + Prettier) — zero errors, zero warnings
- `npm run check` (or `svelte-check`) — TypeScript type errors zero
- `npm run build` — builds cleanly

### Security
- `cargo audit` — no known vulnerabilities in dependencies (run before releases)

### Test Coverage
- `gitv-git-core` crate: ≥ 80% line coverage (measured by `cargo-tarpaulin`)
- The decoupled Git core is the most critical code to test — it has no Tauri deps and is fully mockable

### CI (when set up)
All of the above gates will be enforced in CI. Until then, they are manual developer conventions.

## Toolchain
`mise.toml` declares `npm = "latest"`. Rust toolchain not yet pinned.

## Commit Attribution
AI commits MUST include:
```
Co-Authored-By: <agent-model> <noreply@example.com>
```
