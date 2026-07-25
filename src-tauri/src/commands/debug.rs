use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use tracing::instrument;

static CACHED_MEMORY: AtomicU64 = AtomicU64::new(0);

fn memory_updater() -> &'static std::thread::JoinHandle<()> {
    static HANDLE: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();
    HANDLE.get_or_init(|| {
        if let Some(stats) = memory_stats::memory_stats() {
            CACHED_MEMORY.store(stats.physical_mem as u64, Ordering::Relaxed);
        }
        std::thread::spawn(|| loop {
            std::thread::sleep(std::time::Duration::from_secs(5));
            if let Some(stats) = memory_stats::memory_stats() {
                CACHED_MEMORY.store(stats.physical_mem as u64, Ordering::Relaxed);
            }
        })
    })
}

pub fn start_memory_tracking() {
    memory_updater();
}

/// Returns the current process's physical memory usage in bytes.
/// Value is cached and updated every 5 seconds in the background —
/// this command returns instantly without blocking on syscalls.
#[tauri::command]
#[instrument(fields(command = "get_memory_usage"))]
pub fn get_memory_usage() -> Option<u64> {
    memory_updater();
    let mem = CACHED_MEMORY.load(Ordering::Relaxed);
    if mem == 0 { None } else { Some(mem) }
}
