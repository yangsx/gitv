use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gitv", about = "Modern Git repository visualizer")]
pub struct Cli {
    pub repo_paths: Vec<PathBuf>,

    #[arg(long = "log-level")]
    pub log_level: Option<String>,

    #[arg(long = "debug-overlay")]
    pub debug_overlay: bool,
}

pub fn parse_cli() -> Cli {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        return Cli {
            repo_paths: vec![],
            log_level: None,
            debug_overlay: false,
        };
    }
    Cli::parse_from(args)
}
