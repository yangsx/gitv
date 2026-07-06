/// Memory benchmarks for gitv-git-core
///
/// Uses `dhat` to measure heap allocation during key operations:
///   - CommitInfo model population at 1k / 10k / 100k commits
///   - SearchEngine index construction
///   - GraphCalculator layout computation
///
/// Run with:
///   cargo bench --bench memory_bench
///
/// Output: printed to stdout. dhat also writes a dhat-heap.json profile
/// that can be opened at https://nnethercote.github.io/dh_view/dh_view.html
/// for deeper inspection.

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use chrono::Utc;
use std::collections::HashMap;

use gitv_git_core::graph::{GraphCalculator, GraphOptions};
use gitv_git_core::models::*;
use gitv_git_core::search::{CombineMode, SearchEngine, SearchQuery};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_oid(seed: u64) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..16].copy_from_slice(&seed.to_le_bytes());
    Oid::from_bytes(bytes)
}

fn make_commits(count: usize) -> Vec<CommitInfo> {
    let messages = [
        "fix bug in parser",
        "add new feature",
        "update documentation",
        "refactor module structure",
        "optimize performance",
        "remove deprecated code",
        "add unit tests",
        "fix memory leak",
        "improve error handling",
        "update dependencies",
    ];
    (0..count)
        .map(|i| CommitInfo {
            oid: make_oid(i as u64),
            short_oid: format!("{i:07x}"),
            message: messages[i % messages.len()].into(),
            author: Author {
                name: format!("author{}", i % 10),
                email: format!("a{}@example.com", i % 10),
            },
            committer: Author {
                name: "committer".into(),
                email: "c@example.com".into(),
            },
            author_time: Utc::now(),
            commit_time: Utc::now(),
            parent_oids: if i > 0 {
                vec![make_oid((i - 1) as u64)]
            } else {
                vec![]
            },
            refs: vec![],
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Report helper
// ---------------------------------------------------------------------------

/// Pretty-print a dhat snapshot alongside a label.
fn report(label: &str, stats: &dhat::HeapStats) {
    println!(
        "[memory] {label}\n  \
         total_bytes_allocated : {:>12}\n  \
         total_allocs           : {:>12}\n  \
         peak_bytes             : {:>12}\n  \
         curr_bytes (after)     : {:>12}",
        stats.total_bytes, stats.total_blocks, stats.max_bytes, stats.curr_bytes,
    );
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

/// Measure heap cost of building a Vec<CommitInfo> for different repo sizes.
fn bench_commit_model_memory() {
    for &count in &[1_000usize, 10_000, 100_000] {
        let _profiler = dhat::Profiler::builder().testing().build();
        let _commits = make_commits(count);
        let stats = dhat::HeapStats::get();
        report(&format!("CommitInfo×{count}"), &stats);

        // Sanity assertion: peak usage should stay below 200 MB for 100k commits.
        // Each CommitInfo is roughly ~400–600 bytes of heap.
        if count == 100_000 {
            assert!(
                stats.max_bytes < 200 * 1024 * 1024,
                "CommitInfo×100k peak heap {:.1} MiB exceeds 200 MiB budget",
                stats.max_bytes as f64 / (1024.0 * 1024.0)
            );
        }
    }
}

/// Measure heap cost of building and querying the SearchEngine index.
fn bench_search_engine_memory() {
    for &count in &[1_000usize, 10_000, 100_000] {
        // --- construction ---
        {
            let _profiler = dhat::Profiler::builder().testing().build();
            let commits = make_commits(count);
            let _engine = SearchEngine::new(commits);
            let stats = dhat::HeapStats::get();
            report(&format!("SearchEngine::new×{count}"), &stats);
        }

        // --- query (engine already built, measure incremental allocations) ---
        {
            let commits = make_commits(count);
            let engine = SearchEngine::new(commits);

            let _profiler = dhat::Profiler::builder().testing().build();
            let query = SearchQuery {
                text: Some("fix".into()),
                use_regex: false,
                sha_prefix: None,
                author: None,
                date_range: None,
                file_path: None,
                search_patch: false,
                combine_mode: CombineMode::And,
            };
            let _results = engine.search(&query);
            let stats = dhat::HeapStats::get();
            report(&format!("SearchEngine::search×{count}"), &stats);
        }
    }
}

/// Measure heap cost of graph layout calculation.
fn bench_graph_layout_memory() {
    for &count in &[1_000usize, 10_000, 100_000] {
        let _profiler = dhat::Profiler::builder().testing().build();
        let commits = make_commits(count);
        let calc = GraphCalculator::new(commits, HashMap::new(), vec![], GraphOptions::default());
        let _layout = calc.calculate_layout();
        let stats = dhat::HeapStats::get();
        report(&format!("GraphLayout×{count}"), &stats);

        // Layout for 100k commits should stay below 500 MB peak.
        if count == 100_000 {
            assert!(
                stats.max_bytes < 500 * 1024 * 1024,
                "GraphLayout×100k peak heap {:.1} MiB exceeds 500 MiB budget",
                stats.max_bytes as f64 / (1024.0 * 1024.0)
            );
        }
    }
}

fn main() {
    println!("=== gitv-git-core memory benchmarks ===\n");
    bench_commit_model_memory();
    println!();
    bench_search_engine_memory();
    println!();
    bench_graph_layout_memory();
    println!("\n=== done ===");
}
