use std::path::Path;

/// Progress update for copy/move operations.
#[derive(Debug, Clone)]
pub struct FileProgress {
    /// Current file being processed.
    pub current_file: String,
    /// Number of files completed so far.
    pub files_done: usize,
    /// Total number of files.
    pub files_total: usize,
    /// Total bytes copied so far.
    pub bytes_done: u64,
    /// Total bytes to copy.
    pub bytes_total: u64,
    /// Whether the operation is complete.
    pub finished: bool,
    /// Error message if operation failed.
    pub error: Option<String>,
}

/// Count total files and bytes in a path (recursively for directories).
pub(super) fn count_files_and_bytes(path: &Path) -> (usize, u64) {
    if path.is_dir() {
        let mut count = 0usize;
        let mut bytes = 0u64;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let (c, b) = count_files_and_bytes(&entry.path());
                count += c;
                bytes += b;
            }
        }
        (count, bytes)
    } else {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        (1, size)
    }
}
