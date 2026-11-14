//! Boxes inside a card on a RELIEF theme (neumorphism): pressed into the
//! card as wells, not stroked on it.
//!
//! A soft card has no line round it, yet everything it held was still an
//! outlined box: a settings field, the chat's composer, a file panel (the
//! open list of 2026-11-12: "in-card widgets are still stroked, not
//! wells"). A raised card full of ruled rectangles is half a look. The
//! boxes are read off the cells the pane already draws — a `╭`, the `╮` on
//! its row, and the `╰ ╯` under them — so no pane's layout or hit path
//! changes: the strokes are blanked and a sunk sheet goes under each box,
//! as the input bar's does. A box that holds other boxes (a settings
//! group) loses its rule and gets no sheet: a well inside a well read as a
//! dent in a dent, and its legend heads the wells it holds well enough.
use crew_render::{CellView, PaneScene};

use crate::layout::Rect;

/// How deep an in-card well is pressed: the input bar's depth at rest
/// (`spotlight::composer_lift`), so the two read as one material.
const DEPTH: f32 = -0.65;
/// How far a well's sides stand in from its cells' outer edges, in cells:
/// neighbouring fields keep a rib of card between them.
const SIDE: f32 = 0.25;

/// One box, by the cells of its corners: columns `c0..=c1`, rows `r0..=r1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Box4 {
    pub c0: u16,
    pub r0: u16,
    pub c1: u16,
    pub r1: u16,
}

impl Box4 {
    fn holds(&self, o: &Box4) -> bool {
        self != o && self.c0 <= o.c0 && self.r0 <= o.r0 && self.c1 >= o.c1 && self.r1 >= o.r1
    }
    fn on_edge(&self, c: &CellView) -> bool {
        let inside = (self.c0..=self.c1).contains(&c.col) && (self.r0..=self.r1).contains(&c.row);
        inside && (c.col == self.c0 || c.col == self.c1 || c.row == self.r0 || c.row == self.r1)
    }
}

/// Every round-cornered box drawn in `cells`.
pub(crate) fn boxes(cells: &[CellView]) -> Vec<Box4> {
    let at = |ch: char| -> Vec<(u16, u16)> {
        let mut v: Vec<_> = cells
            .iter()
            .filter(|c| c.c == ch)
            .map(|c| (c.row, c.col))
            .collect();
        v.sort_unstable();
        v
    };
    let (tl, tr, bl, br) = (
        at('\u{256d}'),
        at('\u{256e}'),
        at('\u{2570}'),
        at('\u{256f}'),
    );
    let mut out = Vec::new();
    for &(r0, c0) in &tl {
        let Some(&(_, c1)) = tr.iter().find(|&&(r, c)| r == r0 && c > c0) else {
            continue;
        };
        let Some(&(r1, _)) = bl.iter().find(|&&(r, c)| c == c0 && r > r0) else {
            continue;
        };
        if br.contains(&(r1, c1)) {
            out.push(Box4 { c0, r0, c1, r1 });
        }
    }
    out
}

/// Boxes whose bottom is scrolled out of the pane: a top edge with its two
/// corners and sides running down from them, but no bottom corners. They
/// are unruled like any box and get no sheet (a group is all that scrolls
/// in halves: a field is never cut).
pub(crate) fn cut_boxes(cells: &[CellView]) -> Vec<Box4> {
    let has = |ch: char, row: u16, col: u16| {
        cells
            .iter()
            .any(|c| c.c == ch && c.row == row && c.col == col)
    };
    let mut out = Vec::new();
    for tl in cells.iter().filter(|c| c.c == '\u{256d}') {
        let (r0, c0) = (tl.row, tl.col);
        let tr = cells
            .iter()
            .filter(|c| c.c == '\u{256e}' && c.row == r0 && c.col > c0)
            .map(|c| c.col)
            .min();
        let Some(c1) = tr else { continue };
        let mut r1 = r0;
        while has('\u{2502}', r1 + 1, c0) && has('\u{2502}', r1 + 1, c1) {
            r1 += 1;
        }
        if r1 > r0 && !has('\u{2570}', r1 + 1, c0) {
            out.push(Box4 { c0, r0, c1, r1 });
        }
    }
    out
}

/// The boxes of `all` that become wells: those holding no other box.
pub(crate) fn wells_of(all: &[Box4]) -> Vec<Box4> {
    all.iter()
        .filter(|b| !all.iter().any(|o| b.holds(o)))
        .copied()
        .collect()
}

/// Blank the strokes of `boxes` in `cells`: every rule on a box's edge.
/// A legend standing on the edge stays, a heading inside the well.
pub(crate) fn unstroke(cells: &mut [CellView], wells: &[Box4]) {
    for c in cells.iter_mut() {
        let rule = ('\u{2500}'..='\u{257f}').contains(&c.c);
        if rule && wells.iter().any(|w| w.on_edge(c)) {
            c.c = ' ';
        }
    }
}

/// Where `w`'s sheet lies, for a scene at `(x, y)` with `cw`×`ch` cells.
pub(crate) fn rect_of(w: &Box4, (x, y): (f32, f32), cw: f32, ch: f32) -> Rect {
    Rect {
        x: x + (w.c0 as f32 + SIDE) * cw,
        y: y + w.r0 as f32 * ch,
        w: ((w.c1 - w.c0 + 1) as f32 - 2.0 * SIDE) * cw,
        h: (w.r1 - w.r0 + 1) as f32 * ch,
    }
}

/// Press the boxes of the pane just pushed onto `scenes` (its content,
/// then its card: `paneview::push_pane_scenes`) into wells. A no-op off the
/// relief themes and on a pane with no box.
pub(crate) fn press(scenes: &mut Vec<PaneScene>, cw: f32, ch: f32) {
    if crew_theme::theme().relief() <= 0.0 || scenes.len() < 2 {
        return;
    }
    let at = scenes.len() - 2;
    let (all, cut) = (boxes(&scenes[at].cells), cut_boxes(&scenes[at].cells));
    // A code block is a well too, read off its tint (`relieffield`).
    let (tint, page) = (crate::chatink::code_bg(), crew_theme::theme().page_bg);
    let fields = crate::relieffield::fields(&scenes[at].cells, tint);
    crate::relieffield::clear(&mut scenes[at].cells, &fields, tint, page);
    if all.is_empty() && cut.is_empty() && fields.is_empty() {
        return;
    }
    // A box of several panels is a well a panel (`reliefsplit`); read the
    // joints before the strokes that mark them are blanked.
    let mut wells = fields;
    for w in wells_of(&all) {
        let joints = crate::reliefsplit::joints(&scenes[at].cells, &w);
        crate::reliefsplit::undivide(&mut scenes[at].cells, &w, &joints);
        wells.extend(crate::reliefsplit::panels(&w, &joints));
    }
    unstroke(&mut scenes[at].cells, &all);
    unstroke(&mut scenes[at].cells, &cut);
    let origin = (scenes[at].x, scenes[at].y);
    // After the card: its sheet is drawn first, the wells are sunk into it.
    scenes.extend(
        wells
            .iter()
            .map(|w| crate::navreliefscene::sheet(rect_of(w, origin, cw, ch), cw, ch, DEPTH)),
    );
}

#[cfg(test)]
#[path = "reliefwell_tests.rs"]
mod tests;
