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
#[path = "list_tests.rs"]
mod tests;
