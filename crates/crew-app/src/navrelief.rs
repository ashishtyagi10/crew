//! The open nav on a RELIEF theme (neumorphism): not one tall slab but a
//! column of raised tiles, one per section and one per tab card, each
//! moulded from the page like a pane's card (the user, 2026-11-13: "side
//! panel should also have neumorphism effect").
//!
//! The tiles are read off the cells the nav already draws: a section opens
//! on its rule (`─ TIME ───`, [`crate::boxdraw::section_header`]) and a
//! tab's card on its `╭` ([`crate::tabcard`]). No rule is stroked on a
//! relief theme, so those strokes are blanked and the two shadows say
//! where each tile ends. Rows are untouched: every hit path still reads
//! the same [`crate::navlayout`].
use crew_render::CellView;

use crate::layout::Rect;

/// One raised tile: content rows `top..=bottom` of the nav.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tile {
    pub top: u16,
    pub bottom: u16,
    /// A tab's card: it owns exactly its rows, where a section's tile
    /// spreads into the gap rows around it.
    pub tab: bool,
    /// The tab on screen: its tile stands higher, as a focused pane does.
    pub lifted: bool,
}

const RULE: char = '\u{2500}';
const SIDE: char = '\u{2502}';
const TOP_LEFT: char = '\u{256d}';
const BOTTOM_LEFT: char = '\u{2570}';

/// What stands at column 1 of each row: where rules and tab cards start.
fn leads(cells: &[CellView]) -> Vec<(u16, char)> {
    let mut v: Vec<(u16, char)> = cells
        .iter()
        .filter(|c| c.col == 1)
        .map(|c| (c.row, c.c))
        .collect();
    v.sort_unstable();
    v
}

/// The tiles of a nav drawn as `cells`, top to bottom. A section runs from
/// its rule to the row before the gap that parts it from the next rule; the
/// last one ends on the last row with ink. A rule with a tab's card
/// straight under it (PANES, in tabs) is a heading and gets no tile.
pub(crate) fn tiles(cells: &[CellView]) -> Vec<Tile> {
    let leads = leads(cells);
    let accent = crate::palette::accent();
    let last_ink = cells
        .iter()
        .filter(|c| !c.c.is_whitespace())
        .map(|c| c.row)
        .max()
        .unwrap_or(0);
    let heads: Vec<u16> = leads
        .iter()
        .filter(|(_, c)| *c == RULE)
        .map(|(r, _)| *r)
        .collect();
    let tab_tops: Vec<u16> = leads
        .iter()
        .filter(|(_, c)| *c == TOP_LEFT)
        .map(|(r, _)| *r)
        .collect();
    let mut out = Vec::new();
    for (i, &top) in heads.iter().enumerate() {
        if tab_tops.contains(&(top + 1)) {
            continue;
        }
        let bottom = match heads.get(i + 1) {
            Some(&next) => next.saturating_sub(2).max(top),
            None => last_ink.max(top),
        };
        out.push(Tile {
            top,
            bottom,
            tab: false,
            lifted: false,
        });
    }
    for &top in &tab_tops {
        let bottom = leads
            .iter()
            .find(|(r, c)| *r > top && *c == BOTTOM_LEFT)
            .map_or(last_ink.max(top), |(r, _)| *r);
        let lifted = cells
            .iter()
            .any(|c| c.row == top && c.col == 1 && c.fg == accent);
        out.push(Tile {
            top,
            bottom,
            tab: true,
            lifted,
        });
    }
    out.sort_unstable_by_key(|t| t.top);
    out
}

/// `cells` with the strokes the tiles replace blanked: every section's
/// rule, and each tab card's edges. The words on those rows stay.
pub(crate) fn unruled(cells: &mut [CellView], tiles: &[Tile]) {
    let heads: Vec<u16> = leads(cells)
        .into_iter()
        .filter(|(_, c)| *c == RULE)
        .map(|(r, _)| r)
        .collect();
    let last_col = cells.iter().map(|c| c.col).max().unwrap_or(0);
    for c in cells.iter_mut() {
        let tab = tiles
            .iter()
            .find(|t| t.tab && (t.top..=t.bottom).contains(&c.row));
        let strike = match tab {
            Some(t) if c.row == t.top || c.row == t.bottom => {
                ('\u{2500}'..='\u{257f}').contains(&c.c)
            }
            Some(_) => c.c == SIDE && (c.col == 1 || c.col + 1 >= last_col),
            None => c.c == RULE && heads.contains(&c.row),
        };
        if strike {
            c.c = ' ';
        }
    }
}

/// Where `tile` stands in px: as wide as the nav's own `rect`, its rows
/// from the content's first row (`rect.y + ch`). A section's tile spreads
/// into its gap rows until `gap` px part it from the next, except up from
/// the first row, into the nav's legend. A tab's card stands on rows
/// adjacent to the next card's, so it keeps its top row whole (its name is
/// written there) and gives `gap` px of its last, emptied row.
pub(crate) fn rect_of(tile: &Tile, rect: Rect, ch: f32, gap: f32) -> Rect {
    let spread = ((ch - gap) / 2.0).max(0.0);
    let (up, down) = match tile.tab {
        true => (0.0, -gap.min(ch)),
        false if tile.top == 0 => (0.0, spread),
        false => (spread, spread),
    };
    let y = rect.y + ch + f32::from(tile.top) * ch - up;
    let h = f32::from(tile.bottom - tile.top + 1) * ch + up + down;
    Rect {
        x: rect.x,
        y,
        w: rect.w,
        h: h.max(0.0),
    }
}

#[cfg(test)]
#[path = "navrelief_tests.rs"]
mod tests;
