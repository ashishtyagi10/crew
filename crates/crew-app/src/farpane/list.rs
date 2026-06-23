//! Directory listing for the Far file-manager panels: read entries and sort
//! folders first then files (case-insensitive), with a leading ".." entry
//! whenever the directory has a parent.
use std::path::Path;

use super::Entry;

/// Read `dir` into a sorted entry list: ".." first (unless at the filesystem
/// root), then directories, then files — each group alphabetical and
/// case-insensitive.
pub(crate) fn read_dir(dir: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    if dir.parent().is_some() {
        out.push(Entry {
            name: "..".into(),
            is_dir: true,
            is_parent: true,
        });
    }
    let mut items: Vec<Entry> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            Entry {
                name: e.file_name().to_string_lossy().into_owned(),
                is_dir,
                is_parent: false,
            }
        })
        .collect();
    items.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    out.extend(items);
    out
}

#[cfg(test)]
mod tests {
    use super::read_dir;

    #[test]
    fn lists_parent_first_then_dirs_then_files() {
        let base = std::env::temp_dir().join("crew_far_list_test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("zdir")).unwrap();
        std::fs::create_dir_all(base.join("adir")).unwrap();
        std::fs::write(base.join("bfile.txt"), b"x").unwrap();
        let e = read_dir(&base);
        assert!(e[0].is_parent && e[0].name == "..");
        // directories sort before the file, alphabetically
        assert_eq!(e[1].name, "adir");
        assert_eq!(e[2].name, "zdir");
        assert!(e[1].is_dir && !e[3].is_dir);
        assert_eq!(e[3].name, "bfile.txt");
    }
}
