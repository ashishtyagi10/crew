//! `/disk` — where the space went, as a treemap.
//!
//! `du | sort -n` answers the question in a column of numbers you have to
//! compare by reading. The same directory as areas answers it in one look:
//! the tile that is half the pane IS half the disk.
//!
//! The walk runs on a worker thread and reports partial totals as it goes
//! (see [`crate::project-winit-mainthread-blocking`] rationale: everything in
//! this app runs on the winit thread, so a `read_dir` over a large tree here
//! would freeze every pane). The map redraws while the scan is still running,
//! so a big tree fills in rather than making you wait for a blank pane.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crew_render::{CellView, Paint};

use crate::boxdraw::section_header;
use crate::palette::accent;
use crate::plot::{treemap, Canvas};

/// One child of the scanned directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub name: String,
    pub bytes: u64,
    pub is_dir: bool,
}

/// What the worker fills in as it walks.
#[derive(Default)]
struct Scan {
    children: Mutex<Vec<Child>>,
    /// Files visited so far — the progress readout.
    seen: AtomicU64,
    done: AtomicBool,
    cancel: AtomicBool,
}

pub struct DiskPane {
    root: PathBuf,
    scan: Arc<Scan>,
    /// The last snapshot taken off the worker, sorted descending.
    children: Vec<Child>,
    total: u64,
    scanning: bool,
    files: u64,
    /// The tile the keyboard is on, if any.
    pub(crate) selected: usize,
}

impl DiskPane {
    pub fn new(root: PathBuf) -> Self {
        let mut p = Self {
            root,
            scan: Arc::new(Scan::default()),
            children: Vec::new(),
            total: 0,
            scanning: false,
            files: 0,
            selected: 0,
        };
        p.start();
        p
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Kick off a walk of `self.root` on a worker thread.
    fn start(&mut self) {
        self.scan.cancel.store(true, Ordering::Relaxed); // stop any previous walk
        let scan = Arc::new(Scan::default());
        self.scan = Arc::clone(&scan);
        self.children.clear();
        self.total = 0;
        self.files = 0;
        self.selected = 0;
        self.scanning = true;
        let root = self.root.clone();
        std::thread::spawn(move || walk(&root, &scan));
    }

    /// Rescan the current directory.
    pub fn rescan(&mut self) {
        self.start();
    }

    /// Descend into the selected tile (a directory), or go up with `None`.
    pub fn open(&mut self, child: Option<&str>) {
        match child {
            Some(name) => self.root.push(name),
            None => {
                if !self.root.pop() {
                    return;
                }
            }
        }
        self.start();
    }

    /// The child a tile index names, if it is a directory.
    pub fn child(&self, i: usize) -> Option<&Child> {
        self.children.get(i)
    }

    #[cfg(test)]
    pub fn children(&self) -> &[Child] {
        &self.children
    }

    /// Put a finished scan in without a worker, so the tile renderer can be
    /// exercised on names of a chosen length (the same seam `DashPane` uses).
    #[cfg(test)]
    pub(crate) fn seed_children(&mut self, children: Vec<Child>) {
        self.total = children.iter().map(|c| c.bytes).sum();
        self.files = children.len() as u64;
        self.scanning = false;
        self.children = children;
    }

    /// Take the worker's latest numbers. Returns true when they moved.
    pub fn poll(&mut self) -> bool {
        let seen = self.scan.seen.load(Ordering::Relaxed);
        let done = self.scan.done.load(Ordering::Relaxed);
        let mut changed = seen != self.files || (done && self.scanning);
        self.files = seen;
        if changed {
            if let Ok(list) = self.scan.children.lock() {
                let mut next = list.clone();
                next.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.name.cmp(&b.name)));
                changed = next != self.children;
                self.total = next.iter().map(|c| c.bytes).sum();
                self.children = next;
            }
        }
        if done {
            self.scanning = false;
        }
        changed
    }

    pub fn is_scanning(&self) -> bool {
        self.scanning
    }
}

impl crate::app::CrewApp {
    /// A click on a disk map picks the tile under the pointer — and picks it
    /// *again* to open it, which is how a map is used: point at the big thing,
    /// then go in. Returns true when the click was ours.
    pub(crate) fn disk_click_at_cursor(&mut self) -> bool {
        let Some(i) = self.pane_at_cursor() else {
            return false;
        };
        if !matches!(self.panes[i].content, crate::pane::PaneContent::Disk(_)) {
            return false;
        }
        let Some((row, col)) = self.cursor_rowcol(i) else {
            return false;
        };
        let grid = self.panes[i].grid;
        let crate::pane::PaneContent::Disk(d) = &mut self.panes[i].content else {
            return false;
        };
        self.focused = i;
        self.input.focused = false;
        let Some(hit) = d.tile_at(col as f32, row as f32, grid.cols, grid.rows) else {
            // A click on the map's empty margin still focuses the pane; it
            // just does not move the selection.
            return true;
        };
        if hit == d.selected {
            if let Some(name) = d.child(hit).filter(|c| c.is_dir).map(|c| c.name.clone()) {
                d.open(Some(&name));
            }
        } else {
            d.selected = hit;
        }
        true
    }
}

impl DiskPane {
    /// The tile under a cell coordinate, if any.
    pub fn tile_at(&self, col: f32, row: f32, cols: u16, rows: u16) -> Option<usize> {
        tiles(&self.children, cols, rows)
            .into_iter()
            .find(|t| t.contains(col, row))
            .map(|t| t.index)
    }
}

#[cfg(test)]
impl DiskPane {
    /// Stand in for a completed scan, for tests and the shot harness.
    pub(crate) fn set_children_for_test(&mut self, kids: &[(&str, u64, bool)], selected: usize) {
        self.children = kids
            .iter()
            .map(|(n, b, d)| Child {
                name: (*n).into(),
                bytes: *b,
                is_dir: *d,
            })
            .collect();
        self.total = self.children.iter().map(|c| c.bytes).sum();
        self.scanning = false;
        self.selected = selected;
    }
}

impl Drop for DiskPane {
    /// A closed pane must not keep walking someone's home directory.
    fn drop(&mut self) {
        self.scan.cancel.store(true, Ordering::Relaxed);
    }
}

/// Walk `root`'s children, publishing each child's total as it completes.
fn walk(root: &Path, scan: &Scan) {
    let Ok(dir) = std::fs::read_dir(root) else {
        scan.done.store(true, Ordering::Relaxed);
        return;
    };
    for entry in dir.flatten() {
        if scan.cancel.load(Ordering::Relaxed) {
            return;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let meta = entry.metadata().ok();
        let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
        let bytes = if is_dir {
            dir_size(&entry.path(), scan)
        } else {
            scan.seen.fetch_add(1, Ordering::Relaxed);
            meta.map(|m| m.len()).unwrap_or(0)
        };
        if let Ok(mut list) = scan.children.lock() {
            list.push(Child {
                name,
                bytes,
                is_dir,
            });
        }
    }
    scan.done.store(true, Ordering::Relaxed);
}

/// Bytes under `path`, following no symlinks (a link counts as its own tiny
/// size, not as the tree it points at — otherwise one link into `/` makes the
/// whole map a lie).
fn dir_size(path: &Path, scan: &Scan) -> u64 {
    let mut total = 0;
    let mut stack = vec![path.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if scan.cancel.load(Ordering::Relaxed) {
            return total;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(e.path());
            } else if meta.is_file() {
                total += meta.len();
                scan.seen.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    total
}

/// Bytes as `4.2G`, `812M`, `9.1k`, `640B` — four characters wherever
/// possible, because most of them are written inside a tile.
pub fn bytes(n: u64) -> String {
    const K: f64 = 1024.0;
    let b = n as f64;
    match n {
        0..=1023 => format!("{n}B"),
        _ if b < K * K => format!("{:.0}k", b / K),
        _ if b < K * K * K => format!("{:.0}M", b / (K * K)),
        _ if b < K * K * K * K => format!("{:.1}G", b / (K * K * K)),
        _ if b < K * K * K * K * K => format!("{:.1}T", b / (K * K * K * K)),
        // No real filesystem gets here, but a tile's label must never grow
        // past the box it is written in whatever the number says.
        _ => format!("{:.1}P", b / (K * K * K * K * K)),
    }
}

/// Rows the header claims above the map.
const HEAD: u16 = 2;

/// The map's rect inside a `cols`×`rows` pane, in cells.
fn map_rect(cols: u16, rows: u16) -> (f32, f32, f32, f32) {
    (
        1.0,
        f32::from(HEAD),
        f32::from(cols.saturating_sub(2)),
        f32::from(rows.saturating_sub(HEAD + 1)),
    )
}

/// Tiles for this pane's children, in cell coordinates.
pub fn tiles(children: &[Child], cols: u16, rows: u16) -> Vec<treemap::Tile> {
    let values: Vec<f64> = children.iter().map(|c| c.bytes as f64).collect();
    treemap::layout(map_rect(cols, rows), &values)
}

impl DiskPane {
    pub fn cells(&self, cols: u16, rows: u16) -> Vec<CellView> {
        let t = crew_theme::theme();
        let mut out = Vec::new();
        if cols < 20 || rows < 6 {
            return out;
        }
        out.extend(section_header(
            "DISK",
            cols,
            t.border_normal,
            accent(),
            t.page_bg,
        ));
        let head = match self.scanning {
            true => format!(
                "{}  \u{2014}  {} so far, {} files scanned\u{2026}",
                short_path(&self.root),
                bytes(self.total),
                self.files
            ),
            false => format!(
                "{}  \u{2014}  {} in {} entries",
                short_path(&self.root),
                bytes(self.total),
                self.children.len()
            ),
        };
        put(&mut out, &head, 1, 1, t.ink, cols);

        // A label per tile that has the room for one: name on the first row,
        // size under it. A tile too small for its own name gets none — the
        // area is still the reading.
        for tile in tiles(&self.children, cols, rows) {
            let Some(child) = self.children.get(tile.index) else {
                continue;
            };
            if tile.w < 5.0 || tile.h < 1.0 {
                continue;
            }
            let selected = tile.index == self.selected;
            let fg = if selected { t.ink } else { t.page_bg };
            let room = (tile.w - 1.0) as usize;
            // `vend` is not a directory anybody has. A tile that cuts a name
            // without saying so reads as a complete, wrong name; `ven…` reads
            // as a name that did not fit — which is the truth.
            let name = crate::chatwidth::clip_w(&child.name, room);
            put(
                &mut out,
                &name,
                tile.x as u16 + 1,
                tile.y as u16,
                fg,
                cols.saturating_sub(1),
            );
            if tile.h >= 2.0 {
                put(
                    &mut out,
                    &bytes(child.bytes),
                    tile.x as u16 + 1,
                    tile.y as u16 + 1,
                    fg,
                    cols.saturating_sub(1),
                );
            }
        }
        let hint =
            "\u{2190}\u{2192} pick \u{00b7} enter opens \u{00b7} backspace up \u{00b7} r rescans";
        put(
            &mut out,
            hint,
            1,
            rows.saturating_sub(1),
            t.text_muted,
            cols,
        );
        out
    }

    pub fn paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        let t = crew_theme::theme();
        if cols < 20 || rows < 6 || self.children.is_empty() {
            return Vec::new();
        }
        let mut c = Canvas::new(cols, rows, aspect);
        for tile in tiles(&self.children, cols, rows) {
            let Some(child) = self.children.get(tile.index) else {
                continue;
            };
            // Colour by the roster's tag pool, keyed by name — the same
            // directory keeps its colour across a rescan and between panes,
            // and the pool is already tuned for this page.
            let color = crate::chatroster::agent_color(&child.name);
            let (x, y) = (tile.x, tile.y * aspect);
            let (w, h) = (
                (tile.w - 0.08).max(0.05),
                (tile.h * aspect - 0.08).max(0.05),
            );
            let selected = tile.index == self.selected;
            // Directories carry their colour; plain files sit back, so a tree
            // full of one big file still reads as different from a subtree.
            let alpha = match (child.is_dir, selected) {
                (_, true) => 1.0,
                (true, false) => 0.85,
                (false, false) => 0.55,
            };
            c.rect(x, y, w, h, color, alpha);
            if selected {
                // A ring around the picked tile, drawn as four thin bars: the
                // fill alone cannot say "this one" on a busy map.
                let k = 0.2;
                let ink = t.ink;
                c.rect(x, y, w, k, ink, 1.0);
                c.rect(x, y + h - k, w, k, ink, 1.0);
                c.rect(x, y, k, h, ink, 1.0);
                c.rect(x + w - k, y, k, h, ink, 1.0);
            }
        }
        c.paint()
    }
}

/// What a key press asked the app to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskAction {
    Close,
    Redraw,
}

impl DiskPane {
    /// The map's keys: pick a tile, open it, go up, rescan, close.
    ///
    /// Left/Right walk the tiles in size order rather than by position — the
    /// order the list is already in, and the order you care about: the next
    /// key takes you to the next biggest thing.
    pub fn on_key(&mut self, event: &winit::event::KeyEvent) -> Option<DiskAction> {
        use winit::keyboard::{Key, NamedKey};
        if !event.state.is_pressed() {
            return None;
        }
        match &event.logical_key {
            Key::Named(NamedKey::Escape) => Some(DiskAction::Close),
            Key::Named(NamedKey::ArrowRight) | Key::Named(NamedKey::ArrowDown) => {
                if self.children.is_empty() {
                    return None;
                }
                self.selected = (self.selected + 1) % self.children.len();
                Some(DiskAction::Redraw)
            }
            Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowUp) => {
                if self.children.is_empty() {
                    return None;
                }
                self.selected = self
                    .selected
                    .checked_sub(1)
                    .unwrap_or(self.children.len() - 1);
                Some(DiskAction::Redraw)
            }
            Key::Named(NamedKey::Enter) => {
                // Only a directory can be descended into; a file tile is the
                // end of the road and says so by not moving.
                let name = self.child(self.selected).filter(|c| c.is_dir)?.name.clone();
                self.open(Some(&name));
                Some(DiskAction::Redraw)
            }
            Key::Named(NamedKey::Backspace) => {
                self.open(None);
                Some(DiskAction::Redraw)
            }
            Key::Character(c) if c.as_str() == "r" => {
                self.rescan();
                Some(DiskAction::Redraw)
            }
            _ => None,
        }
    }
}

fn put(out: &mut Vec<CellView>, s: &str, col: u16, row: u16, fg: (u8, u8, u8), cols: u16) {
    for (i, ch) in s.chars().enumerate() {
        let col = col + i as u16;
        if col >= cols {
            break;
        }
        out.push(CellView {
            col,
            row,
            c: ch,
            fg,
            bg: crew_theme::theme().page_bg,
            ..Default::default()
        });
    }
}

/// `~/code/crew` rather than `/Users/you/code/crew`.
fn short_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    match dirs::home_dir() {
        Some(home) => match s.strip_prefix(home.to_string_lossy().as_ref()) {
            Some(rest) => format!("~{rest}"),
            None => s.into_owned(),
        },
        None => s.into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{bytes, tiles, Child, DiskPane};

    fn kids(sizes: &[(&str, u64, bool)]) -> Vec<Child> {
        sizes
            .iter()
            .map(|(n, b, d)| Child {
                name: (*n).into(),
                bytes: *b,
                is_dir: *d,
            })
            .collect()
    }

    #[test]
    fn a_tiles_share_of_the_pane_is_its_share_of_the_bytes() {
        let c = kids(&[("big", 800, true), ("small", 200, true)]);
        let t = tiles(&c, 40, 20);
        let area = |i: usize| {
            let t = t.iter().find(|t| t.index == i).unwrap();
            t.w * t.h
        };
        let ratio = area(0) / (area(0) + area(1));
        assert!((ratio - 0.8).abs() < 0.02, "80% of the bytes: {ratio}");
    }

    #[test]
    fn bytes_are_four_characters_wherever_they_can_be() {
        assert_eq!(bytes(0), "0B");
        assert_eq!(bytes(900), "900B");
        assert_eq!(bytes(9_216), "9k");
        assert_eq!(bytes(5 * 1024 * 1024), "5M");
        assert_eq!(bytes(4_509_715_660), "4.2G");
        assert!(bytes(u64::MAX).len() <= 8, "{}", bytes(u64::MAX));
    }

    #[test]
    fn the_selection_wraps_in_size_order() {
        let mut p = DiskPane::new(std::env::temp_dir());
        p.children = kids(&[("a", 3, true), ("b", 2, true), ("c", 1, false)]);
        assert_eq!(p.selected, 0);
        p.selected = 2;
        // Past the end wraps to the biggest again; the order is the list's,
        // which is size order.
        p.selected = (p.selected + 1) % p.children.len();
        assert_eq!(p.selected, 0);
        p.selected = p.selected.checked_sub(1).unwrap_or(p.children.len() - 1);
        assert_eq!(p.selected, 2);
    }

    #[test]
    fn only_a_directory_can_be_descended_into() {
        let mut p = DiskPane::new(std::env::temp_dir());
        p.children = kids(&[("dir", 3, true), ("file.txt", 2, false)]);
        assert!(p.child(0).filter(|c| c.is_dir).is_some());
        assert!(p.child(1).filter(|c| c.is_dir).is_none());
    }

    #[test]
    fn walking_up_from_the_root_stays_at_the_root() {
        let mut p = DiskPane::new(std::path::PathBuf::from("/"));
        p.open(None);
        assert_eq!(p.root(), std::path::Path::new("/"));
    }

    #[test]
    fn a_real_directory_is_walked_and_totalled() {
        // The one test that actually touches the filesystem: a scan has to
        // produce the sizes it claims, and the pane has to notice.
        let dir = std::env::temp_dir().join(format!("crew-disk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.bin"), vec![0u8; 4096]).unwrap();
        std::fs::write(dir.join("sub").join("b.bin"), vec![0u8; 1024]).unwrap();

        let mut p = DiskPane::new(dir.clone());
        // The walk is on a worker thread; poll until it reports done.
        let start = std::time::Instant::now();
        while p.is_scanning() && start.elapsed() < std::time::Duration::from_secs(5) {
            p.poll();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        p.poll();
        assert!(!p.is_scanning(), "the scan finished");
        let mut names: Vec<(&str, u64)> = p
            .children()
            .iter()
            .map(|c| (c.name.as_str(), c.bytes))
            .collect();
        names.sort();
        assert_eq!(names, vec![("a.bin", 4096), ("sub", 1024)]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_click_lands_on_the_tile_it_is_over() {
        let mut p = DiskPane::new(std::env::temp_dir());
        p.set_children_for_test(&[("big", 800, true), ("small", 200, true)], 0);
        let t = tiles(p.children(), 40, 20);
        for tile in &t {
            let (cx, cy) = (tile.x + tile.w / 2.0, tile.y + tile.h / 2.0);
            assert_eq!(p.tile_at(cx, cy, 40, 20), Some(tile.index));
        }
        // The header rows are not the map.
        assert_eq!(p.tile_at(1.0, 0.0, 40, 20), None);
    }

    #[test]
    fn a_pane_too_small_for_the_map_draws_nothing_rather_than_a_mess() {
        let p = DiskPane::new(std::env::temp_dir());
        assert!(p.cells(12, 20).is_empty());
        assert!(p.paint(12, 20, 2.0).is_empty());
    }

    /// A treemap tile that cuts a name without saying so reads as a complete,
    /// wrong name: `vendor` in a small tile drew `vend`, which is not a
    /// directory anybody has.
    #[test]
    fn a_tile_too_narrow_for_a_name_says_the_name_is_cut() {
        let _g = crate::app::theme_test_guard();
        let mut p = DiskPane::new(std::env::temp_dir());
        // One huge tile and one small one, so the small tile is narrow.
        p.seed_children(kids(&[("target", 760, true), ("vendor", 240, true)]));
        let text = |cells: &[crew_render::CellView]| -> String {
            let mut v: Vec<&crew_render::CellView> = cells.iter().collect();
            v.sort_by_key(|c| (c.row, c.col));
            v.iter().map(|c| c.c).collect()
        };
        let drawn = text(&p.cells(26, 14));
        assert!(
            drawn.contains("target"),
            "the big tile keeps its whole name: {drawn:?}"
        );
        // The narrow tile marks its cut rather than drawing `vend`, which
        // would read as a complete name for a directory nobody has.
        assert!(
            drawn.contains("ven\u{2026}"),
            "a cut name drew as a complete wrong one: {drawn:?}"
        );
        assert!(
            !drawn.contains("vendor"),
            "it really did not fit: {drawn:?}"
        );
    }
}
