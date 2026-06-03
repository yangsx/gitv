# Agent Instructions

## Project Status
**Pre-implementation.** No source code yet. `design.md` (root) is the active design; `.kiro/specs/gitv/` holds the original requirements and superseded React-based design for reference.

## Architecture
- **Rust + Tauri 2.0** desktop app: Git visualization tool (modern gitk)
- **Backend**: `src-tauri/` — Rust with `gix` (gitoxide) for Git ops, `wgpu` for GPU graph rendering
- **Decoupled Git core**: `crates/gitv-git-core/` — pure Rust, no Tauri deps, independently testable
- **Frontend**: `src/` — SvelteKit + Svelte 5 + TypeScript + Tailwind + Vite
- **Pure Rust preferred**: use pure-Rust crates over C bindings; shell out to `git` CLI only as documented fallback

## Planned Project Layout
```
src-tauri/           # Rust backend (Tauri commands in src/commands/)
crates/gitv-git-core/ # Git logic crate (repository, graph, search, stream, watcher, models)
src/                 # SvelteKit frontend (routes/, lib/components/, lib/stores/, lib/actions/, lib/bindings/)
tests/               # Integration tests + fixtures
benches/             # criterion benchmarks
```

## Key Design Decisions
- Git backend is a separate crate (`gitv-git-core`) with trait-based interfaces for mocking
- Commit graph uses GPU-accelerated rendering via wgpu with virtualized viewport
- Commits are streamed in batches (not loaded all at once) to keep UI responsive
- Each tab holds isolated repository state (own connection, filters, scroll position)
- Branch filtering happens at Git traversal layer, not UI layer

## Tech Stack
| Layer | Tool |
|-------|------|
| App framework | Tauri 2.0 |
| Git library | gix (gitoxide) |
| GPU rendering | wgpu |
| FS watching | notify + notify-debouncer-full |
| Frontend | Svelte 5, TypeScript, Svelte stores |
| Virtual list | svelte-virtual-scroll-list |
| Styling | Tailwind CSS |
| Build (frontend) | Vite |
| Test runner (Rust) | cargo-nextest |
| Benchmarks | criterion |
| Coverage | cargo-tarpaulin |

## Toolchain
`mise.toml` declares `npm = "latest"`. Rust toolchain not yet pinned.

## Commit Attribution
AI commits MUST include:
```
Co-Authored-By: <agent-model> <noreply@example.com>
```
