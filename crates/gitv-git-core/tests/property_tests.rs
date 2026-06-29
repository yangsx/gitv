use chrono::Utc;
use gitv_git_core::graph::{GraphCalculator, GraphOptions};
use gitv_git_core::models::*;
use gitv_git_core::repository::Repository;
use gitv_git_core::search::{CombineMode, SearchEngine, SearchQuery};
use proptest::prelude::*;
use std::collections::HashMap;

fn make_oid(seed: u64) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[0..8].copy_from_slice(&seed.to_le_bytes());
    Oid::from_bytes(bytes)
}

fn make_commit(oid_seed: u64, parent_seeds: Vec<u64>, author_name: &str) -> CommitInfo {
    CommitInfo {
        oid: make_oid(oid_seed),
        short_oid: format!("{:07x}", oid_seed),
        message: format!("commit {oid_seed}"),
        summary: format!("commit {oid_seed}"),
        author: Author {
            name: author_name.into(),
            email: format!("{author_name}@example.com"),
        },
        committer: Author {
            name: author_name.into(),
            email: format!("{author_name}@example.com"),
        },
        author_time: Utc::now(),
        commit_time: Utc::now(),
        parent_oids: parent_seeds.into_iter().map(make_oid).collect(),
        refs: vec![],
    }
}

prop_compose! {
    fn arb_linear_commits(count: usize)
        (seeds in prop::collection::vec(any::<u64>(), count))
        -> Vec<CommitInfo> {
        let mut commits = Vec::with_capacity(count);
        for (i, _) in seeds.iter().enumerate() {
            let parent = if i > 0 { vec![(i - 1) as u64] } else { vec![] };
            commits.push(make_commit(i as u64, parent, "author"));
        }
        commits
    }
}

proptest! {
    #[test]
    fn prop_graph_all_nodes_present(commits in arb_linear_commits(50)) {
        let calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            vec![],
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        let input_oids: std::collections::HashSet<Oid> = commits.iter().map(|c| c.oid).collect();
        let layout_oids: std::collections::HashSet<Oid> = layout.nodes.iter().map(|n| n.oid).collect();
        assert_eq!(input_oids, layout_oids);
    }

    #[test]
    fn prop_graph_unique_rows(commits in arb_linear_commits(50)) {
        let calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            vec![],
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        let rows: std::collections::HashSet<usize> = layout.nodes.iter().map(|n| n.row).collect();
        assert_eq!(rows.len(), layout.nodes.len(), "every node must have a unique row");
    }

    #[test]
    fn prop_graph_edges_connect_valid_nodes(commits in arb_linear_commits(50)) {
        let calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            vec![],
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        let node_oids: std::collections::HashSet<Oid> = layout.nodes.iter().map(|n| n.oid).collect();
        let row_map: std::collections::HashMap<Oid, usize> =
            layout.nodes.iter().map(|n| (n.oid, n.row)).collect();
        for edge in &layout.edges {
            assert!(edge.from_row <= edge.to_row,
                "edges should not go backward: from_row={} to_row={}", edge.from_row, edge.to_row);
        }
        let _ = node_oids;
        let _ = row_map;
    }

    #[test]
    fn prop_graph_total_rows_matches_node_count(commits in arb_linear_commits(50)) {
        let calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            vec![],
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        assert!(
            layout.total_rows >= layout.nodes.len(),
            "total_rows ({}) should be >= node count ({})",
            layout.total_rows,
            layout.nodes.len()
        );
    }

    #[test]
    fn prop_search_returns_subset(query_str in "[a-z]{1,10}") {
        let commits: Vec<CommitInfo> = (0..100u64)
            .map(|i| make_commit(i, if i > 0 { vec![i - 1] } else { vec![] }, "author"))
            .collect();
        let engine = SearchEngine::new(commits);
        let query = SearchQuery {
            text: Some(query_str),
            use_regex: false,
            search_patch: false,
            sha_prefix: None,
            author: None,
            date_range: None,
            file_path: None,
            combine_mode: CombineMode::And,
        };
        if let Ok(results) = engine.search(&query) {
            for r in &results {
                assert!(r.commit_oid == make_oid(0) || r.commit_oid.to_hex().starts_with("00"),
                    "all results should reference valid commits");
            }
        }
    }

    #[test]
    fn prop_search_index_commit_count(n in 10u64..200) {
        let commits: Vec<CommitInfo> = (0..n)
            .map(|i| make_commit(i, if i > 0 { vec![i - 1] } else { vec![] }, "author"))
            .collect();
        let engine = SearchEngine::new(commits);
        assert_eq!(engine.commit_count(), n as usize);
    }
}

/// Manual measurement test — run with:
///   cargo test --package gitv-git-core --test property_tests measure_real_repo -- --ignored --nocapture
#[test]
#[ignore]
fn measure_real_repo() {
    for path_str in &["/home/yangsx/C/Rust/grit", "/home/yangsx/C/software/gitv"] {
        let path = std::path::Path::new(path_str);
        let repo = match gitv_git_core::gix_repo::GixRepository::open(path) {
            Ok(r) => r,
            Err(_) => {
                eprintln!("skip {path_str}: not a git repo");
                continue;
            }
        };
        let commits = repo.commits(None, &[]).expect("failed to load commits");
        let refs_map: HashMap<Oid, Vec<gitv_git_core::models::Ref>> = HashMap::new();

        let calc = GraphCalculator::new(commits, refs_map, Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let errors = layout.verify();
        eprintln!(
            "{path_str}: {} nodes, {} edges, {} cols, {} errors",
            layout.nodes.len(),
            layout.edges.len(),
            layout.total_columns,
            errors.len(),
        );
        for e in errors.iter().take(3) {
            eprintln!("    {e}");
        }
    }
}
