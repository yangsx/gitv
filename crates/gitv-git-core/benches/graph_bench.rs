use std::collections::HashMap;

use chrono::Utc;
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use gitv_git_core::graph::{GraphCalculator, GraphOptions};
use gitv_git_core::models::*;

fn make_oid(seed: u64) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0..8].copy_from_slice(&seed.to_le_bytes());
    bytes[8..16].copy_from_slice(&seed.to_le_bytes());
    Oid::from_bytes(bytes)
}

fn generate_linear_commits(count: usize) -> Vec<CommitInfo> {
    let mut commits = Vec::with_capacity(count);
    for i in 0..count {
        let oid = make_oid(i as u64);
        let parent_oids = if i > 0 {
            vec![make_oid((i - 1) as u64)]
        } else {
            vec![]
        };
        commits.push(CommitInfo {
            oid,
            short_oid: format!("{:07x}", i),
            message: format!("commit {i}"),
            summary: format!("commit {i}"),
            author: Author {
                name: "author".into(),
                email: "a@example.com".into(),
            },
            committer: Author {
                name: "author".into(),
                email: "a@example.com".into(),
            },
            author_time: Utc::now(),
            commit_time: Utc::now(),
            parent_oids,
            refs: vec![],
        });
    }
    commits
}

fn generate_branchy_commits(count: usize, branch_freq: usize) -> Vec<CommitInfo> {
    let mut commits = Vec::with_capacity(count);
    let mut branch_tips: Vec<Oid> = Vec::new();
    for i in 0..count {
        let oid = make_oid(i as u64);
        let parent_oids = if i == 0 {
            vec![]
        } else if i % branch_freq == 0 {
            let branch_point = if i > branch_freq {
                make_oid((i - branch_freq) as u64)
            } else {
                make_oid(0u64)
            };
            branch_tips.push(oid);
            vec![branch_point]
        } else if !branch_tips.is_empty() && i % (branch_freq * 3) == 0 {
            let merge_from = branch_tips.pop().unwrap();
            let linear_parent = make_oid((i - 1) as u64);
            vec![linear_parent, merge_from]
        } else {
            vec![make_oid((i - 1) as u64)]
        };
        commits.push(CommitInfo {
            oid,
            short_oid: format!("{:07x}", i),
            message: format!("commit {i}"),
            summary: format!("commit {i}"),
            author: Author {
                name: format!("author{}", i % 5),
                email: format!("a{}@example.com", i % 5),
            },
            committer: Author {
                name: format!("author{}", i % 5),
                email: format!("a{}@example.com", i % 5),
            },
            author_time: Utc::now(),
            commit_time: Utc::now(),
            parent_oids,
            refs: vec![],
        });
    }
    commits
}

fn bench_graph_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("graph_layout");
    for size in [100, 1_000, 10_000] {
        let commits = generate_linear_commits(size);
        group.bench_with_input(BenchmarkId::new("linear", size), &commits, |b, commits| {
            b.iter(|| {
                let calc = GraphCalculator::new(
                    black_box(commits.clone()),
                    HashMap::new(),
                    vec![],
                    GraphOptions::default(),
                );
                black_box(calc.calculate_layout())
            });
        });
    }
    group.finish();

    let mut group = c.benchmark_group("graph_layout_branchy");
    for size in [100, 1_000, 10_000] {
        let commits = generate_branchy_commits(size, 20);
        group.bench_with_input(BenchmarkId::new("branchy", size), &commits, |b, commits| {
            b.iter(|| {
                let calc = GraphCalculator::new(
                    black_box(commits.clone()),
                    HashMap::new(),
                    vec![],
                    GraphOptions::default(),
                );
                black_box(calc.calculate_layout())
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_graph_layout);
criterion_main!(benches);
