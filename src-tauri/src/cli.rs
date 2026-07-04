use clap::Parser;
use std::path::PathBuf;

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

    /// Maximum commits to process in self-test mode (default: no limit).
    #[arg(long = "max-commits", value_name = "N")]
    pub self_test_max_commits: Option<usize>,
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
            self_test_max_commits: None,
        };
    }
    Cli::parse_from(args)
}
