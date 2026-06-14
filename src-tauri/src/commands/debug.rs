use tracing::instrument;

/// Returns the current process's physical memory usage in bytes.
#[tauri::command]
#[instrument(fields(command = "get_memory_usage"))]
pub fn get_memory_usage() -> Option<u64> {
    memory_stats::memory_stats().map(|m| m.physical_mem as u64)
}
