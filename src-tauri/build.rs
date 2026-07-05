fn main() {
    tauri_build::build();

    // Force re-run when git HEAD moves
    let git_dir = ".git";
    if let Ok(head) = std::fs::read_to_string(format!("{git_dir}/HEAD")) {
        println!("cargo:rerun-if-changed={git_dir}/HEAD");
        if let Some(ref_path) = head.strip_prefix("ref: ") {
            println!("cargo:rerun-if-changed={git_dir}/{}", ref_path.trim());
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
