# Agent Instructions

## Project Status
**In development.** Core application has substantial source code in place:
- **Backend** (`src-tauri/`): Tauri commands for preferences, graph layout, commits, diff, repo operations, saved searches, file watching, diagnostics
- **Git core** (`crates/gitv-git-core/`): 77 tests passing — repository abstraction, graph calculator/layout, search engine (RoaringBitmap), streaming, file watching, disk cache, models
- **Frontend** (`frontend/`): Svelte 5 + TypeScript with CommitGraph (canvas-based), CommitList, CommitDetailPanel (diff/whitespace controls), PreferencesModal (draggable), Toolbar, SearchBar, Sidebar, FileTree, BlamePanel, CommandPalette, ContextMenu, DebugOverlay
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
frontend/             # SvelteKit frontend (src/routes/, src/lib/components/, src/lib/stores/, src/lib/actions/, src/lib/bindings/)
tests/                # Integration tests + fixtures
benches/              # criterion benchmarks
```

## Key Design Decisions
- Git backend is a separate crate (`gitv-git-core`) with trait-based interfaces for mocking
- `Oid` is a 20-byte binary newtype (`[u8; 20]`), not a String — 3x less memory, faster hashing
- Commit graph uses GPU-accelerated rendering via wgpu with virtualized viewport
- Commits are streamed in batches with binary serialization (postcard) to avoid JSON IPC overhead
- Persistent disk cache for graph layout + metadata — re-open cached repos in <200ms
- Each tab holds isolated repository state (own connection, filters, scroll position)
- Branch filtering happens at Git traversal layer, not UI layer
- SvelteKit with static adapter — single-window desktop app uses `+page.svelte`/`+layout.svelte` conventions; static adapter produces a SPA for Tauri
- Stashes displayed as single markers on parent commit rows in the graph (not gitk's two-node double-diff display); combined diff by default with optional staged/unstaged split toggle (Req 38)
- Diff viewer supports normal, word-diff, and stat-only modes with whitespace modifiers (Req 54)
- Graph supports color-by-author mode (Req 52), merge filtering (Req 53), commit dimming (Req 56), and orientation toggle (Req 57)
- CLI accepts revision ranges (`gitv /repo v1.0..v2.0`) and filter flags (Req 55)
- CLI argument parsing via `clap`
- Panel widths/heights are clamped on restore to min/max bounds — prevents unusable layouts from tiling WMs (Req 59)
- Structured tracing via `tracing` crate with rolling file logs; debug overlay with FPS/memory/IPC timing (Reqs 68-69)
- Crash diagnostics captured via panic hook with backtrace, retained as crash logs (Req 70)
- Preferences persisted as JSON at `$XDG_CONFIG_HOME/gitv/preferences.json` with atomic writes; debounced frontend auto-save (300ms) avoids excessive I/O
- Diff/whitespace controls in CommitDetailPanel use local `$state` overrides (not global preference saves); PreferencesModal sets global defaults
- Preferences dialog is draggable (no backdrop) to allow flexible placement alongside the graph
- Graph toolbar simplified — toggle buttons (color mode, hide merges, orientation) moved into Preferences dialog; only gear icon remains on toolbar

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
- `cargo clippy -- -D warnings` — zero warnings, zero errors
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
