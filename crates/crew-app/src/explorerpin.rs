//! The Explorer's roots in the app: the focused pane's folder first, then the pinned ones
//! ([`crate::explorerroots`]) — and the ways a folder is pinned and unpinned: `/nav explorer
//! add|remove <path>`, a pinned root's `×`, Delete on its row with the tree holding the keys.
use std::path::{Path, PathBuf};

use crate::app::CrewApp;
use crate::explorer::Item;
use crate::pathexpand::{expand_path, normalized};

/// What `/nav explorer` takes besides opening the section.
pub(crate) const USAGE: &str = "/nav explorer add|remove <folder or file>";

impl CrewApp {
    /// Every root the tree shows: the pane's, then each pinned folder — save one that IS the
    /// pane's root just now, which would draw the same tree twice.
    pub(crate) fn explorer_roots(&self) -> Vec<PathBuf> {
        let here = self.explorer_root();
        let same = normalized(&here);
        let pinned = self.sidebar.explorer.pinned();
        let mut out = vec![here];
        out.extend(pinned.into_iter().filter(|p| *p != same));
        out
    }

    /// The tree's rows: each root's tree, one after the other.
    pub(crate) fn explorer_items(&self) -> Vec<Item> {
        let e = &self.sidebar.explorer;
        self.explorer_roots()
            .iter()
            .flat_map(|r| e.items(r))
            .collect()
    }

    /// The pinned root at row `k` of `items`, if that is what it is: every root after the
    /// first is pinned.
    pub(crate) fn explorer_pinned_at(items: &[Item], k: usize) -> Option<&Path> {
        let i = items.get(k).filter(|i| k > 0 && i.depth == 0)?;
        Some(i.path.as_path())
    }

    /// The pinned root whose `×` is under the pointer, as a row of [`Self::explorer_items`].
    pub(crate) fn explorer_x_at_cursor(&self) -> Option<usize> {
        // The column first: it costs no walk of the tree, and most moves are not on it.
        if !self.nav_x_under_cursor() {
            return None;
        }
        let k = self.explorer_item_at_cursor()?;
        Self::explorer_pinned_at(&self.explorer_items(), k).map(|_| k)
    }

    /// The tree's hover as the repaint check compares it: the row, and whether its `×` is lit
    /// — sliding onto the `×` changes no row but must repaint.
    pub(crate) fn explorer_hover_code(&self) -> Option<usize> {
        // The rule's `+` lights too: a code no row makes.
        if self.explorer_add_at_cursor() {
            return Some(usize::MAX - 1);
        }
        let k = self.explorer_item_at_cursor()?;
        let lit = self.nav_x_under_cursor() && self.explorer_x_at_cursor().is_some();
        Some(k * 2 + usize::from(lit))
    }

    /// Unpin the root on row `k`, if it is a pinned one. `true` when it was.
    pub(crate) fn explorer_unpin_at(&mut self, k: usize) -> bool {
        let items = self.explorer_items();
        let Some(dir) = Self::explorer_pinned_at(&items, k).map(Path::to_path_buf) else {
            return false;
        };
        self.sidebar.explorer.unpin(&dir);
        self.set_status(format!("explorer: unpinned {}", short(&dir)));
        self.redraw();
        true
    }

    /// `/nav explorer add|remove <path>`. A path is read as `/view` reads one, and as the bar
    /// completes it: `~` and `$VAR` expanded, a relative one against where you are. A FILE pins its folder and opens in
    /// the tree's viewer — the credentials file you came for, not just where it lives.
    pub(crate) fn explorer_pin_command(&mut self, arg: &str) {
        let (verb, rest) = arg.split_once(char::is_whitespace).unwrap_or((arg, ""));
        let rest = rest.trim();
        let add = match verb {
            "add" | "pin" => true,
            "remove" | "rm" | "unpin" => false,
            _ => return self.set_status(format!("usage: {USAGE}")),
        };
        if rest.is_empty() {
            // `add` alone asks the system's chooser for the folder, as the rule's `+` does.
            return match add {
                true => self.explorer_choose_folder(),
                false => self.set_status(format!("usage: {USAGE}")),
            };
        }
        let base = match self.cwd.as_os_str().is_empty() {
            true => self.explorer_root(),
            false => self.cwd.clone(),
        };
        let path = normalized(&expand_path(&base, rest));
        if !add {
            let msg = match self.sidebar.explorer.unpin(&path) {
                true => "unpinned",
                false => "not pinned:",
            };
            self.set_status(format!("explorer: {msg} {}", short(&path)));
            return self.redraw();
        }
        self.explorer_pin_path(path);
    }

    /// Pin `path` (absolute and normalized) and show it: a folder as a root of the tree, a
    /// file's folder with the file open.
    pub(crate) fn explorer_pin_path(&mut self, path: PathBuf) {
        let (dir, file) = match () {
            _ if path.is_dir() => (path, None),
            _ if path.is_file() => match path.parent() {
                Some(d) => (d.to_path_buf(), Some(path.clone())),
                None => (path.clone(), None),
            },
            _ => return self.set_status_err(format!("explorer: no such path {}", short(&path))),
        };
        let e = &self.sidebar.explorer;
        let msg = match () {
            _ if dir == normalized(&self.explorer_root()) => "already the tree's root:",
            _ if e.is_pinned(&dir) => "already pinned:",
            _ if e.pin(&dir) => "pinned",
            _ => "too many pinned folders \u{2014} unpin one first; not pinned:",
        };
        self.set_status(format!("explorer: {msg} {}", short(&dir)));
        // Shown: the section opens on the tree, scrolled to the folder or open on the file.
        self.config.show_nav = true;
        self.config.nav_collapsed = false;
        self.config.nav_section = crate::navsection::NavSection::Explorer.as_str().into();
        self.config.save();
        self.sidebar.explorer.shut.remove(&dir);
        // A viewer already open on a file in it is revealed there on the next frame.
        self.sidebar.explorer.revealed = None;
        match file {
            Some(f) => {
                self.sidebar.explorer.picked = Some(f.clone());
                self.explorer_open(f);
            }
            None => {
                let items = self.explorer_items();
                if let Some(row) = items.iter().position(|i| i.depth == 0 && i.path == dir) {
                    self.explorer_show_row(row, Some(items.len()));
                }
            }
        }
        self.redraw();
    }
}

fn short(p: &Path) -> String {
    crate::disktile::short_path(p)
}

#[cfg(test)]
#[path = "explorerpin_tests.rs"]
mod tests;
