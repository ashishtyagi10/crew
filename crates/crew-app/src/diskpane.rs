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

use crew_render::CellView;

use crate::plot::treemap;

/// One child of the scanned directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Child {
    pub name: String,
    pub bytes: u64,
    pub is_dir: bool,
}

/// What the worker fills in as it walks.
#[derive(Default)]
pub(crate) struct Scan {
    pub(crate) children: Mutex<Vec<Child>>,
    /// Files visited so far — the progress readout.
    pub(crate) seen: AtomicU64,
    pub(crate) done: AtomicBool,
    pub(crate) cancel: AtomicBool,
}

pub struct DiskPane {
    pub(crate) root: PathBuf,
    pub(crate) scan: Arc<Scan>,
    /// The last snapshot taken off the worker, sorted descending.
    pub(crate) children: Vec<Child>,
    pub(crate) total: u64,
    pub(crate) scanning: bool,
    pub(crate) files: u64,
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
impl DiskPane {}

impl Drop for DiskPane {
    /// A closed pane must not keep walking someone's home directory.
    fn drop(&mut self) {
        self.scan.cancel.store(true, Ordering::Relaxed);
    }
}

/// Walk `root`'s children, publishing each child's total as it completes.
pub(crate) fn walk(root: &Path, scan: &Scan) {
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

impl DiskPane {}

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

pub(crate) fn put(
    out: &mut Vec<CellView>,
    s: &str,
    col: u16,
    row: u16,
    fg: (u8, u8, u8),
    cols: u16,
) {
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

/// The ink a tile's label is written in: the page's own background pushed
/// until it clears the reading floor against `bg`.
///
/// `enforced`, not `against`: a tile's fill is a colour the app composited,
/// not one anybody picked against this page, and the pool has hues that top
/// out short of the floor at every lightness — a file tile at 0.55 alpha over
/// a dark page reached 4.34 and stopped there. `enforced` gives up chroma
/// rather than the floor, so every tile on the map is readable, not most.
pub(crate) fn label_ink(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    crew_theme::readable::enforced(
        crew_theme::theme().page_bg,
        bg,
        crew_theme::contrast::text_floor(),
    )
}

/// Columns the header keeps for the path before it gives up and shows the
/// reading alone — fewer than this and the leading `\u{2026}` is most of it.
pub(crate) const MIN_PATH_W: u16 = 8;

/// The colour a tile's label is read against: its fill composited over the
/// page at the alpha the fill is actually drawn with, which is the background
/// the eye sees — not the raw pool colour, which a 0.55-alpha file tile never
/// shows.
pub(crate) fn tile_bg(color: (u8, u8, u8), is_dir: bool, selected: bool) -> (u8, u8, u8) {
    let page = crew_theme::theme().page_bg;
    let a = tile_alpha(is_dir, selected);
    let mix = |c: u8, p: u8| (f32::from(c) * a + f32::from(p) * (1.0 - a)).round() as u8;
    (
        mix(color.0, page.0),
        mix(color.1, page.1),
        mix(color.2, page.2),
    )
}

/// Whether two tiles share an edge or a corner — laid out edge to edge, so
/// "touching" is what a treemap's neighbours always are.
fn touches(a: &treemap::Tile, b: &treemap::Tile) -> bool {
    const E: f32 = 0.01;
    a.x < b.x + b.w + E && b.x < a.x + a.w + E && a.y < b.y + b.h + E && b.y < a.y + a.h + E
}

/// A colour per tile, chosen so that no two tiles that touch share one.
///
/// The pool is six entries picked by hashing the name, which on a directory
/// with eight children collides by the pigeonhole principle long before it
/// collides by bad luck: in the repo's own root `crates` and `.git` came out
/// byte-identical, and so did `target` and `docs`. Two neighbouring tiles the
/// same colour read as one region, which is the one thing a map of areas is
/// for. On a single-phosphor tube, where the whole pool is four shades of the
/// same green, it is the only thing keeping the regions apart at all.
///
/// The name still picks first, so a directory keeps its colour when you
/// rescan the parent it is in; only a tile that would touch a twin steps
/// along the pool to the next free entry.
pub(crate) fn tile_colors(children: &[Child], tiles: &[treemap::Tile]) -> Vec<(u8, u8, u8)> {
    let pool: Vec<(u8, u8, u8)> = {
        let t = crew_theme::theme();
        let mut v: Vec<(u8, u8, u8)> = Vec::new();
        for c in &t.ansi[9..=14] {
            if !v.contains(c) {
                v.push(*c);
            }
        }
        v
    };
    let mut picked: Vec<Option<usize>> = vec![None; tiles.len()];
    for (i, tile) in tiles.iter().enumerate() {
        let Some(child) = children.get(tile.index) else {
            continue;
        };
        let first = pool
            .iter()
            .position(|&c| c == crate::chatroster::agent_color(&child.name))
            .unwrap_or(0);
        // The neighbours that already have one. Tiles are visited in the
        // layout's own order, so this is deterministic for a given listing.
        let taken: Vec<usize> = tiles
            .iter()
            .enumerate()
            .filter(|&(j, t)| j != i && touches(tile, t))
            .filter_map(|(j, _)| picked[j])
            .collect();
        picked[i] = Some(
            (0..pool.len())
                .map(|d| (first + d) % pool.len())
                // A tile with more neighbours than the pool has colours keeps
                // the one its name asked for: a repeat somewhere beats a
                // colour nothing chose.
                .find(|k| !taken.contains(k))
                .unwrap_or(first),
        );
    }
    picked.into_iter().map(|k| pool[k.unwrap_or(0)]).collect()
}

/// How solid a tile is drawn. Directories carry their colour; plain files sit
/// back, so a tree full of one big file still reads as different from a
/// subtree — and the picked tile is solid whichever it is.
pub(crate) fn tile_alpha(is_dir: bool, selected: bool) -> f32 {
    match (is_dir, selected) {
        (_, true) => 1.0,
        (true, false) => 0.85,
        (false, false) => 0.55,
    }
}

/// `~/code/crew` rather than `/Users/you/code/crew`.
pub(crate) fn short_path(p: &Path) -> String {
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
#[path = "diskpane_tests.rs"]
mod tests;
