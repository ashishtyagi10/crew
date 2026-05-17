use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A group of duplicate files sharing the same content.
#[derive(Debug, Clone)]
pub struct DuplicateGroup {
    /// SHA-256 hash (hex) shared by all files in this group.
    pub hash: String,
    /// Size of each file in bytes.
    pub size: u64,
    /// Paths of all duplicate files.
    pub paths: Vec<PathBuf>,
}

/// Scan a directory for duplicate files.
/// Uses a two-pass approach: group by size, then hash only size-collisions.
pub fn find_duplicates(root: &Path, max_depth: usize) -> Result<Vec<DuplicateGroup>> {
    // Pass 1: group files by size
    let mut by_size: HashMap<u64, Vec<PathBuf>> = HashMap::new();
    scan_dir(root, &mut by_size, 0, max_depth)?;

    // Keep only sizes with multiple files (potential duplicates)
    by_size.retain(|_, paths| paths.len() > 1);

    // Pass 2: hash files that share a size
    let mut by_hash: HashMap<String, (u64, Vec<PathBuf>)> = HashMap::new();
    for (size, paths) in &by_size {
        for path in paths {
            match hash_file(path) {
                Ok(hash) => {
                    by_hash
                        .entry(hash)
                        .or_insert_with(|| (*size, Vec::new()))
                        .1
                        .push(path.clone());
                }
                Err(_) => continue,
            }
        }
    }

    // Keep only actual duplicates (same hash, 2+ files)
    let mut groups: Vec<DuplicateGroup> = by_hash
        .into_iter()
        .filter(|(_, (_, paths))| paths.len() > 1)
        .map(|(hash, (size, paths))| DuplicateGroup { hash, size, paths })
        .collect();

    // Sort by total reclaimable space (largest first)
    groups.sort_by(|a, b| {
        let a_waste = a.size * (a.paths.len() as u64 - 1);
        let b_waste = b.size * (b.paths.len() as u64 - 1);
        b_waste.cmp(&a_waste)
    });

    Ok(groups)
}

fn scan_dir(
    dir: &Path,
    by_size: &mut HashMap<u64, Vec<PathBuf>>,
    depth: usize,
    max_depth: usize,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() && meta.len() > 0 {
                by_size.entry(meta.len()).or_default().push(path);
            } else if meta.is_dir() {
                scan_dir(&path, by_size, depth + 1, max_depth)?;
            }
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Format a human-readable size.
pub fn format_size(size: u64) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_duplicates_groups_by_content_not_only_size() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("a.txt"), b"same").unwrap();
        std::fs::write(root.join("b.txt"), b"same").unwrap();
        std::fs::write(root.join("c.txt"), b"diff").unwrap();

        let groups = find_duplicates(root, 5).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].paths.len(), 2);
        let names: Vec<String> = groups[0]
            .paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"b.txt".to_string()));
    }

    #[test]
    fn find_duplicates_respects_max_depth() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::create_dir(root.join("d1")).unwrap();
        std::fs::create_dir(root.join("d1").join("d2")).unwrap();
        std::fs::write(root.join("top.bin"), b"x").unwrap();
        std::fs::write(root.join("d1").join("mid.bin"), b"x").unwrap();
        std::fs::write(root.join("d1").join("d2").join("deep.bin"), b"x").unwrap();

        let shallow = find_duplicates(root, 0).unwrap();
        assert!(shallow.is_empty());

        let depth1 = find_duplicates(root, 1).unwrap();
        assert_eq!(depth1.len(), 1);
        assert_eq!(depth1[0].paths.len(), 2);
    }

    #[test]
    fn duplicate_groups_sorted_by_reclaimable_space() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();

        // Group 1 waste: (10 * (3-1)) = 20
        std::fs::write(root.join("g1a"), vec![b'a'; 10]).unwrap();
        std::fs::write(root.join("g1b"), vec![b'a'; 10]).unwrap();
        std::fs::write(root.join("g1c"), vec![b'a'; 10]).unwrap();

        // Group 2 waste: (20 * (2-1)) = 20 (tie)
        std::fs::write(root.join("g2a"), vec![b'b'; 20]).unwrap();
        std::fs::write(root.join("g2b"), vec![b'b'; 20]).unwrap();

        let groups = find_duplicates(root, 1).unwrap();
        assert_eq!(groups.len(), 2);
        let wastes: Vec<u64> = groups
            .iter()
            .map(|g| g.size * (g.paths.len() as u64 - 1))
            .collect();
        assert!(wastes[0] >= wastes[1]);
    }

    #[test]
    fn format_size_outputs_expected_units() {
        assert_eq!(format_size(999), "999 B");
        assert_eq!(format_size(1_024), "1.0 KB");
        assert_eq!(format_size(1_048_576), "1.0 MB");
    }
}
