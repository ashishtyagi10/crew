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
            !name.starts_with('.')
                && !(e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()))
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
mod tests {
    use super::*;

    /// Build a throwaway tree under the OS temp dir; unique per test run.
    fn fixture(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("crew-fileindex-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("target/debug")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("README.md"), "hi").unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("src/.hidden"), "x").unwrap();
        std::fs::write(dir.join("target/debug/junk"), "x").unwrap();
        std::fs::write(dir.join(".git/config"), "x").unwrap();
        dir
    }

    #[test]
    fn scan_lists_files_relative_and_sorted() {
        let dir = fixture("basic");
        let files = scan(&dir);
        assert_eq!(
            files,
            vec!["README.md".to_string(), "src/main.rs".to_string()]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_skips_hidden_and_build_dirs() {
        let dir = fixture("skips");
        let files = scan(&dir);
        assert!(!files
            .iter()
            .any(|f| f.contains(".git") || f.contains("target") || f.contains(".hidden")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_of_missing_dir_is_empty() {
        assert!(scan(Path::new("/nonexistent/definitely-not-here")).is_empty());
    }
}
