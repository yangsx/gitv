use chrono::Utc;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use gitv_git_core::models::*;
use gitv_git_core::search::{CombineMode, SearchEngine, SearchQuery};

fn make_oid(seed: u64) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..16].copy_from_slice(&seed.to_le_bytes());
    Oid::from_bytes(bytes)
}

fn generate_commits_for_search(count: usize) -> Vec<CommitInfo> {
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
    let mut commits = Vec::with_capacity(count);
    for i in 0..count {
        commits.push(CommitInfo {
            oid: make_oid(i as u64),
            short_oid: format!("{:07x}", i),
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
            parent_oids: vec![],
            refs: vec![],
        });
    }
    commits
}

fn bench_search_index_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_index_build");
    for size in [1_000, 10_000, 100_000] {
        let commits = generate_commits_for_search(size);
        group.bench_with_input(BenchmarkId::new("build", size), &commits, |b, commits| {
            b.iter(|| black_box(SearchEngine::new(black_box(commits.clone()))));
        });
    }
    group.finish();
}

fn bench_search_query(c: &mut Criterion) {
    let commits = generate_commits_for_search(100_000);
    let engine = SearchEngine::new(commits);

    let mut group = c.benchmark_group("search_query_100k");
    group.bench_function("text_exact", |b| {
        b.iter(|| {
            let query = SearchQuery {
                text: Some("fix".into()),
                use_regex: false,
                combine_mode: CombineMode::And,
                ..Default::default()
            };
            black_box(engine.search(black_box(&query)))
        });
    });
    group.bench_function("text_regex", |b| {
        b.iter(|| {
            let query = SearchQuery {
                text: Some("fix|add".into()),
                use_regex: true,
                combine_mode: CombineMode::And,
                ..Default::default()
            };
            black_box(engine.search(black_box(&query)))
        });
    });
    group.bench_function("author", |b| {
        b.iter(|| {
            let query = SearchQuery {
                text: None,
                use_regex: false,
                author: Some("author5".into()),
                combine_mode: CombineMode::And,
                ..Default::default()
            };
            black_box(engine.search(black_box(&query)))
        });
    });
    group.bench_function("sha_prefix", |b| {
        b.iter(|| {
            let query = SearchQuery {
                text: None,
                use_regex: false,
                sha_prefix: Some("00000000".into()),
                combine_mode: CombineMode::And,
                ..Default::default()
            };
            black_box(engine.search(black_box(&query)))
        });
    });
    group.finish();
}

criterion_group!(benches, bench_search_index_build, bench_search_query);
criterion_main!(benches);
