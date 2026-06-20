//! Small free-function helpers used across the app module: byte sizes,
//! disk-space probing, and recursive directory size.

use std::path::Path;

/// Recursively calculate the total size of a directory.
pub(super) fn dir_size_recursive(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    total += dir_size_recursive(&entry.path());
                } else {
                    total += meta.len();
                }
            }
        }
    }
    total
}

/// Format a byte count into a human-readable size string.
pub(super) fn format_size_human(size: u64) -> String {
    if size < 1_000 {
        format!("{} B", size)
    } else if size < 1_000_000 {
        format!("{:.1} KB", size as f64 / 1_024.0)
    } else if size < 1_000_000_000 {
        format!("{:.1} MB", size as f64 / 1_048_576.0)
    } else {
        format!("{:.2} GB", size as f64 / 1_073_741_824.0)
    }
}

/// Get free and total disk space for the given path (unix-only; returns
/// `(None, None)` on other platforms).
#[allow(dead_code)] // Plan 4: status bar disk-space display removed in Plan 2 T5
pub(super) fn get_disk_space_cached(path: &Path) -> (Option<u64>, Option<u64>) {
    #[cfg(unix)]
    {
        use std::ffi::CString;
        let c_path = CString::new(path.to_string_lossy().as_bytes()).ok();
        if let Some(c_path) = c_path {
            unsafe {
                let mut stat: libc::statvfs = std::mem::zeroed();
                if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
                    let free = stat.f_bavail as u64 * stat.f_frsize;
                    let total = stat.f_blocks as u64 * stat.f_frsize;
                    return (Some(free), Some(total));
                }
            }
        }
        (None, None)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        (None, None)
    }
}
