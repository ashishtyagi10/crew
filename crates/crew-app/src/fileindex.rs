//! Bounded file listing for chat `@file` mentions: a walkdir scan of the app
//! cwd, capped in depth and count so the winit thread never stalls, skipping
//! hidden entries and heavyweight build dirs.
use std::path::Path;

/// Most files collected per scan; fuzzy filtering still works over a
/// truncated set, and the cap bounds the main-thread stall.
pub(crate) const MAX_FILES: usize = 2000;
const MAX_DEPTH: usize = 8;
/// Directories that are never worth mentioning and often huge.
const SKIP_DIRS: [&str; 3] = ["target", "node_modules", ".git"];

/// List files under `root` as sorted, `/`-separated relative paths. Bounded
/// (depth, count, skip list) — see the module doc; errors are skipped.
pub(crate) fn scan(root: &Path) -> Vec<String> {
    let walker = walkdir::WalkDir::new(root)
        .max_depth(MAX_DEPTH)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            let skipped_dir = e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref());
            !name.starts_with('.') && !skipped_dir
        });
    let mut files = Vec::new();
    for entry in walker.flatten() {
        if files.len() >= MAX_FILES {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(root) {
            files.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }
    files
}

#[cfg(test)]
#[path = "fileindex_tests.rs"]
mod tests;
