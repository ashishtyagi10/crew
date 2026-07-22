//! Directory listing for the Far file-manager panels: read entries and sort
//! folders first (case-insensitive) then files largest-first, with a leading
//! ".." entry whenever the directory has a parent.
use std::path::Path;

use super::Entry;

/// Read `dir` into a sorted entry list: ".." first (unless at the filesystem
/// root), then directories alphabetical and case-insensitive, then files by
/// size descending (name as the tiebreaker).
pub(crate) fn read_dir(dir: &Path) -> Vec<Entry> {
    let mut out = Vec::new();
    if dir.parent().is_some() {
        out.push(Entry {
            name: "..".into(),
            is_dir: true,
            is_parent: true,
            size: 0,
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
                size: if is_dir {
                    0
                } else {
                    e.metadata().map(|m| m.len()).unwrap_or(0)
                },
            }
        })
        .collect();
    sort_entries(&mut items);
    out.extend(items);
    out
}

/// Sort a listing folders-first, then files largest-first, name as tiebreak
/// (case-insensitive). Shared by the local reader and remote `lsjson`
/// parsing so both panels order identically.
pub(crate) fn sort_entries(items: &mut [Entry]) {
    items.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| b.size.cmp(&a.size))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
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

    #[test]
    fn files_sort_by_size_descending_with_name_tiebreak() {
        let base = std::env::temp_dir().join("crew_far_size_sort_test");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("dir")).unwrap();
        std::fs::write(base.join("small.txt"), b"x").unwrap();
        std::fs::write(base.join("big.txt"), vec![b'x'; 500]).unwrap();
        std::fs::write(base.join("also-small.txt"), b"y").unwrap();
        let e = read_dir(&base);
        // ".." then the dir, then files largest-first; equal sizes by name.
        let names: Vec<&str> = e.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            ["..", "dir", "big.txt", "also-small.txt", "small.txt"]
        );
        assert_eq!(e[2].size, 500);
        assert_eq!(e[1].size, 0, "directories carry no size");
        assert_eq!(e[0].size, 0, "the parent row carries no size");
    }
}
