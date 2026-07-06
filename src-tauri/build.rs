fn main() {
    tauri_build::build();

    // Walk up from crate dir to find the .git directory (it may be at the
    // workspace root, not next to the crate).
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let mut current = std::path::PathBuf::from(&manifest_dir);
    let git_dir = loop {
        let candidate = current.join(".git");
        if candidate.exists() {
            break Some(candidate);
        }
        if !current.pop() {
            break None;
        }
    };

    // Force re-run when git HEAD moves
    if let Some(ref git_dir) = git_dir {
        let head_path = git_dir.join("HEAD");
        if let Ok(head) = std::fs::read_to_string(&head_path) {
            println!("cargo:rerun-if-changed={}", head_path.display());
            if let Some(ref_path) = head.strip_prefix("ref: ") {
                println!(
                    "cargo:rerun-if-changed={}",
                    git_dir.join(ref_path.trim()).display()
                );
            }
        }
    }

    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GITV_COMMIT_SHA={sha}");
}
