//! The Explorer section: the working directory as a tree, the way VS Code's explorer shows a
//! workspace.
//!
//! The root is the focused pane's directory — where the shell you are typing in is standing —
//! or crew's own when the pane has none. Folders open and close on a click and stay open across
//! root changes (they are remembered by path), so walking between panes and back finds the tree
//! as you left it.
//!
//! **Reading the disk.** A folder is listed when it is first opened — on the click, which is
//! where a wait belongs — and refreshed on a worker thread whenever its listing is older than
//! [`STALE`], the old listing drawn meanwhile: a file an agent just wrote appears within a couple
//! of seconds without a watcher, and a folder of thousands (`target/debug/deps` reads in ~10 ms)
//! never stalls a frame to say so. Listings are capped at [`CAP`] entries; past it the folder says how many it is not
//! showing rather than drawing a list nobody can scroll through.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How old a listing may get before it is read again.
const STALE: Duration = Duration::from_secs(2);
/// Most entries one folder lists.
pub(crate) const CAP: usize = 2000;

/// One entry of a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub dir: bool,
}

struct Listing {
    at: Instant,
    /// Shared, not copied: the tree is flattened several times a frame.
    entries: std::rc::Rc<Vec<Entry>>,
    /// Entries past [`CAP`] that were not kept.
    more: usize,
}

/// One row of the tree as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    /// 0 for the root, 1 for its children, and so on.
    pub depth: u16,
    pub name: String,
    pub path: PathBuf,
    pub dir: bool,
    pub open: bool,
    /// `Some(n)`: not an entry but the "n more" line closing a capped folder.
    pub more: Option<usize>,
}

/// The tree's state: what is open, what was read, how far it is scrolled.
#[derive(Default)]
pub(crate) struct Explorer {
    /// Folders below a root that are open, by path.
    pub(crate) open: HashSet<PathBuf>,
    /// Roots the user closed — a root starts open.
    pub(crate) shut: HashSet<PathBuf>,
    listings: RefCell<HashMap<PathBuf, Listing>>,
    /// Rows scrolled off the top.
    pub scroll: usize,
    /// The file drawn marked: the one last opened from the tree, or the focused viewer's
    /// ([`crate::explorerreveal`]).
    pub picked: Option<PathBuf>,
    /// The focused pane (`born_ms`) and its viewer's file the tree last revealed — so the
    /// reveal runs once per change, and a scroll after it is the user's to keep.
    pub(crate) revealed: Option<(u64, Option<PathBuf>)>,
    /// The tree's hold on the keyboard ([`crate::explorerkeys`]).
    pub keys: Option<crate::explorerkeys::TreeKeys>,
    /// The name being typed to jump to while it does ([`crate::explorerfind`]).
    pub find: crate::explorerfind::Find,
    /// The root last shown: kept while the focused pane has no directory of its own — a viewer
    /// the tree itself just opened must not swing the tree to somewhere else.
    pub(crate) root: RefCell<Option<PathBuf>>,
    /// The viewer pane the tree opens files in (its `born_ms`), replaced by the next file
    /// opened — VS Code's preview editor — rather than a pane per click.
    pub preview: Option<u64>,
    /// Refreshes in flight, and where their listings come back.
    pending: RefCell<HashSet<PathBuf>>,
    back: RefCell<Option<Chan>>,
}

type Fresh = (PathBuf, Vec<Entry>, usize);
type Chan = (
    std::sync::mpsc::Sender<Fresh>,
    std::sync::mpsc::Receiver<Fresh>,
);

impl Explorer {
    /// The tree under `root`, flattened into the rows it draws as.
    pub(crate) fn items(&self, root: &Path) -> Vec<Item> {
        let open = !self.shut.contains(root);
        let name = root.file_name().map_or_else(
            || root.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let mut out = vec![Item {
            depth: 0,
            name,
            path: root.to_path_buf(),
            dir: true,
            open,
            more: None,
        }];
        if open {
            self.walk(root, 1, &mut out);
        }
        out
    }

    fn walk(&self, dir: &Path, depth: u16, out: &mut Vec<Item>) {
        let (entries, more) = self.listing(dir);
        for e in entries.iter() {
            let open = e.dir && self.open.contains(&e.path);
            out.push(Item {
                depth,
                name: e.name.clone(),
                path: e.path.clone(),
                dir: e.dir,
                open,
                more: None,
            });
            if open {
                self.walk(&e.path, depth + 1, out);
            }
        }
        if more > 0 {
            out.push(Item {
                depth,
                name: format!("\u{2026} {more} more"),
                path: dir.to_path_buf(),
                dir: false,
                open: false,
                more: Some(more),
            });
        }
    }

    /// `dir`'s entries, read now if they never were or have gone stale.
    fn listing(&self, dir: &Path) -> (std::rc::Rc<Vec<Entry>>, usize) {
        self.land();
        let mut cache = self.listings.borrow_mut();
        match cache.get(dir).map(|l| l.at.elapsed() < STALE) {
            Some(true) => {}
            // Stale: serve what was read, and read again off the frame thread.
            Some(false) => self.refresh(dir),
            None => {
                let (entries, more) = read(dir);
                let entries = std::rc::Rc::new(entries);
                let at = Instant::now();
                cache.insert(dir.to_path_buf(), Listing { at, entries, more });
            }
        }
        let l = &cache[dir];
        (std::rc::Rc::clone(&l.entries), l.more)
    }

    /// Open a closed folder, close an open one. A folder that closes forgets its listing (and
    /// its children's), so the cache holds what is on screen and no more.
    pub(crate) fn toggle(&mut self, item: &Item) {
        if item.open {
            let mut cache = self.listings.borrow_mut();
            cache.retain(|p, _| !p.starts_with(&item.path) || p == &item.path && item.depth == 0);
        }
        let flip = |set: &mut HashSet<PathBuf>| {
            if !set.remove(&item.path) {
                set.insert(item.path.clone());
            }
        };
        match item.depth {
            0 => flip(&mut self.shut),
            _ => flip(&mut self.open),
        }
    }

    /// Scroll by `lines` (positive: back toward the top), kept inside a list of `len` rows shown
    /// `rows` at a time.
    pub(crate) fn scroll_by(&mut self, lines: i32, len: usize, rows: usize) {
        let max = len.saturating_sub(rows) as i64;
        self.scroll = (self.scroll as i64 - i64::from(lines)).clamp(0, max) as usize;
    }
}

#[path = "explorerdisk.rs"]
mod disk;
pub(crate) use disk::read;

#[cfg(test)]
#[path = "explorer_tests.rs"]
pub(crate) mod tests;
