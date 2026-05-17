use std::path::{Path, PathBuf};

/// Recursively scan files starting at `dir`, collecting `(abs_path, name, rel_path)`
/// entries into `out`. Skips hidden files. Bounded by depth and total count.
pub(super) fn scan_files(
    out: &mut Vec<(PathBuf, String, String)>,
    dir: &Path,
    root: &Path,
    depth: usize,
) {
    if depth > 8 || out.len() > 10_000 {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        out.push((path.clone(), name, rel));
        if path.is_dir() {
            scan_files(out, &path, root, depth + 1);
        }
    }
}
