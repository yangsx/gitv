use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gitv", about = "Modern Git repository visualizer")]
pub struct Cli {
    pub repo_paths: Vec<PathBuf>,
}

pub fn parse_cli() -> Vec<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        return vec![];
    }
    Cli::parse_from(args).repo_paths
}
