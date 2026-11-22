//! Reading the disk for the Explorer: one folder's listing, and the worker that refreshes a
//! stale one off the frame thread. Split from [`super`] for the line cap.
use std::path::Path;
use std::time::Instant;

use super::{Entry, Explorer, Listing, CAP};

/// Names never listed: version-control internals and Finder's droppings.
const HIDDEN: [&str; 2] = [".git", ".DS_Store"];

impl Explorer {
    /// Read `dir` again on a worker, unless a read of it is already out.
    pub(super) fn refresh(&self, dir: &Path) {
        if !self.pending.borrow_mut().insert(dir.to_path_buf()) {
            return;
        }
        let mut back = self.back.borrow_mut();
        let (tx, _) = back.get_or_insert_with(std::sync::mpsc::channel);
        let (tx, dir) = (tx.clone(), dir.to_path_buf());
        std::thread::spawn(move || {
            let (entries, more) = read(&dir);
            let _ = tx.send((dir, entries, more));
        });
    }

    /// Take in the listings the workers have finished.
    pub(super) fn land(&self) {
        let back = self.back.borrow();
        let Some((_, rx)) = back.as_ref() else {
            return;
        };
        let mut cache = self.listings.borrow_mut();
        for (dir, entries, more) in rx.try_iter() {
            self.pending.borrow_mut().remove(&dir);
            let entries = std::rc::Rc::new(entries);
            cache.insert(
                dir,
                Listing {
                    at: Instant::now(),
                    entries,
                    more,
                },
            );
        }
    }
}

/// A folder's entries, folders first, each group by name ignoring case; and how many past
/// [`CAP`] were dropped. A folder that cannot be read lists nothing.
pub(crate) fn read(dir: &Path) -> (Vec<Entry>, usize) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (Vec::new(), 0);
    };
    let mut v: Vec<Entry> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if HIDDEN.contains(&name.as_str()) {
                return None;
            }
            let path = e.path();
            // The entry's own type costs nothing (it came with the listing); only a link is
            // followed, with a stat, to learn whether it is a folder you can open.
            let dir = match e.file_type() {
                Ok(t) if t.is_symlink() => path.is_dir(),
                Ok(t) => t.is_dir(),
                Err(_) => false,
            };
            Some(Entry { name, path, dir })
        })
        .collect();
    // Each name lowercased once, not once per comparison.
    v.sort_by_cached_key(|e| (!e.dir, e.name.to_lowercase()));
    let more = v.len().saturating_sub(CAP);
    v.truncate(CAP);
    (v, more)
}
