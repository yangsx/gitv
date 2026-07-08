use clap::Parser;
use std::path::PathBuf;

use gitv_git_core::graph::GraphOrientation;

#[derive(Parser)]
#[command(
    name = "gitv",
    about = "Modern Git repository visualizer",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GITV_COMMIT_SHA"), ")")
)]
pub struct Cli {
    pub repo_paths: Vec<PathBuf>,

    #[arg(long = "log-level")]
    pub log_level: Option<String>,

    #[arg(long = "debug-overlay")]
    pub debug_overlay: bool,

    /// Run headless graph self-test (human-readable summary to stderr).
    #[arg(long = "self-test", value_name = "PATH")]
    pub self_test: Option<PathBuf>,

    /// Run headless graph self-test (JSON output to stdout).
    #[arg(long = "self-test-json", value_name = "PATH")]
    pub self_test_json: Option<PathBuf>,

    /// Dump computed graph layout as text (nodes, edges, expanded paths).
    #[arg(long = "dump-graph", value_name = "PATH")]
    pub dump_graph: Option<PathBuf>,

    /// Dump computed graph layout as JSON (full layout + commits + diagnostics).
    #[arg(long = "dump-graph-json", value_name = "PATH")]
    pub dump_graph_json: Option<PathBuf>,

    /// Maximum commits to process in self-test/dump mode (default: no limit).
    #[arg(long = "max-commits", value_name = "N")]
    pub max_commits: Option<usize>,

    /// Hide merge commits in dump output (applies to --dump-graph / --dump-graph-json only).
    #[arg(long = "hide-merges")]
    pub hide_merges: bool,

    /// Graph orientation in dump output: top-to-bottom or bottom-to-top
    /// (applies to --dump-graph / --dump-graph-json only).
    #[arg(long = "orientation", value_name = "DIR")]
    pub orientation: Option<String>,
}

pub fn parse_cli() -> Cli {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        return Cli {
            repo_paths: vec![],
            log_level: None,
            debug_overlay: false,
            self_test: None,
            self_test_json: None,
            dump_graph: None,
            dump_graph_json: None,
            max_commits: None,
            hide_merges: false,
            orientation: None,
        };
    }
    Cli::parse_from(args)
}

/// Parse the --orientation CLI string into a GraphOrientation.
/// Returns Err with a helpful message on invalid input.
pub fn parse_orientation(s: &str) -> Result<GraphOrientation, String> {
    match s.to_lowercase().as_str() {
        "top-to-bottom" | "ttb" | "top" => Ok(GraphOrientation::TopToBottom),
        "bottom-to-top" | "btt" | "bottom" => Ok(GraphOrientation::BottomToTop),
        _ => Err(format!(
            "invalid orientation '{s}' — expected: top-to-bottom, bottom-to-top, ttb, btt"
        )),
    }
}
