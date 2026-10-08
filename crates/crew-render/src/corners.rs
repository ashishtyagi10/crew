//! Card-scale corners: every frame corner rounds at the radius the cards'
//! glass does, not the one a single cell can hold.
//!
//! A `╭` drawn in its own cell can bend no further than half that cell, so
//! every card crew drew — and every box a program drew in a terminal — was a
//! square with its corners nicked (the user, 2026-10-08: "make all corner
//! rounded"). Here a corner borrows the next cell along each arm: the corner
//! glyph and the first `─` and `│` it reaches into are blanked, and in their
//! place one quarter arc is drawn — a ring segment, clipped to those cells,
//! with straight tails out to their far edges where the untouched rules
//! carry on.
//!
//! The tails have to meet those rules to the pixel, and a rule glyph lands
//! wherever cosmic-text and glyphon put it: left at the cell origin snapped
//! by cosmic-text's subpixel bins, top at the origin's whole pixel plus the
//! line's top rounded up. So each patch is placed from the very buffer the
//! rules are drawn from ([`cell_at`]), and the band it continues is the one
//! its neighbouring rule actually fills.
//!
//! A corner whose arms are not plain rules — a legend or a mark standing
//! where the arc would run — keeps its glyph.
use std::collections::HashSet;

use crate::cellgrid::CellView;
use crate::roundborder::Border;
use crate::scene::{PaneBuffer, PaneScene};
use crate::stretch::Split;

const RULE_H: char = '\u{2500}';
const RULE_V: char = '\u{2502}';

/// The way a corner's arms run, `(dx, dy)`: `╭` reaches right and down. The
/// square corners round too — a box is a box, whoever drew it.
fn arms(c: char) -> Option<(i32, i32)> {
    match c {
        '\u{256D}' | '\u{250C}' => Some((1, 1)),
        '\u{256E}' | '\u{2510}' => Some((-1, 1)),
        '\u{256F}' | '\u{2518}' => Some((-1, -1)),
        '\u{2570}' | '\u{2514}' => Some((1, -1)),
        _ => None,
    }
}

/// The characters of a scene by position: one flat grid, so a terminal
/// pane's ten thousand cells cost one allocation a frame, not a map.
struct Chars {
    cols: i32,
    rows: i32,
    c: Vec<(char, (u8, u8, u8))>,
}

impl Chars {
    fn of(cells: &[CellView]) -> Self {
        let cols = cells
            .iter()
            .map(|c| i32::from(c.col) + 1)
            .max()
            .unwrap_or(0);
        let rows = cells
            .iter()
            .map(|c| i32::from(c.row) + 1)
            .max()
            .unwrap_or(0);
        let mut c = vec![('\0', (0, 0, 0)); (cols * rows) as usize];
        for cell in cells {
            c[(i32::from(cell.row) * cols + i32::from(cell.col)) as usize] = (cell.c, cell.fg);
        }
        Self { cols, rows, c }
    }

    fn cell(&self, col: i32, row: i32) -> Option<(char, (u8, u8, u8))> {
        let inside = (0..self.cols).contains(&col) && (0..self.rows).contains(&row);
        inside
            .then(|| self.c[(row * self.cols + col) as usize])
            .filter(|&(c, _)| c != '\0')
    }

    fn get(&self, col: i32, row: i32) -> Option<char> {
        self.cell(col, row).map(|(c, _)| c)
    }
}

/// A frame's light stroke in a `cell_w`×`cell_h` cell, as `boxglyph` draws
/// it: `(thickness, left of the vertical rule, top of the horizontal one)`.
fn stroke(cell_w: f32, cell_h: f32) -> (f32, f32, f32) {
    let (cw, ch) = (cell_w.round() as u32, cell_h.round() as u32);
    let (lx, hx) = crate::boxglyph::band(cw, 1, ch);
    let (ly, _) = crate::boxglyph::band(ch, 1, ch);
    ((hx - lx) as f32, lx as f32, ly as f32)
}

/// How far a quarter circle of radius `r` stands off its straight rule at
/// `d` px along it from the corner.
fn bow(r: f32, d: f32) -> f32 {
    let k = (r - d).max(0.0);
    r - (r * r - k * k).max(0.0).sqrt()
}

/// The radius (px, at the stroke's centre) every card corner rounds to —
/// and the glass sheet under it, so the frame and its glass bend as one.
///
/// As large as one borrowed cell allows: from the stroke's centre to the far
/// edge of the first cell along the arm, a pixel short so the tail meets the
/// next rule at full ink. Then held off the card's first cell of content —
/// the arc bows inward, and it must not reach the text.
pub fn radius(cell_w: f32, cell_h: f32) -> f32 {
    let (t, lx, ly) = stroke(cell_w, cell_h);
    let (cw, ch) = (cell_w.round(), cell_h.round());
    // The tighter side of the cell, whichever way the corner faces.
    let near_x = (lx + t / 2.0).min(cw - lx - t / 2.0);
    let near_y = (ly + t / 2.0).min(ch - ly - t / 2.0);
    let mut r = (near_x + cw - 1.0).min(near_y + ch - 1.0);
    while r > 1.0
        && (bow(r, near_x) + t / 2.0 > near_y - 1.0 || bow(r, near_y) + t / 2.0 > near_x - 1.0)
    {
        r -= 0.5;
    }
    r.max(1.0)
}

/// One corner that rounds: where its glyph is, which way it faces, how many
/// cells each arm lends it, and the colours at the ends of its arms — the
/// arc fades from one to the other, so a gradient ring stays a gradient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Corner {
    pub col: u16,
    pub row: u16,
    pub dx: i32,
    pub dy: i32,
    pub nx: u16,
    pub ny: u16,
    pub fg_h: (u8, u8, u8),
    pub fg_v: (u8, u8, u8),
}

impl Corner {
    /// Every cell the arc is drawn over: the corner and its borrowed arms.
    fn cells(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (c, r) = (i32::from(self.col), i32::from(self.row));
        let h = (1..=i32::from(self.nx)).map(move |k| (c + self.dx * k, r));
        let v = (1..=i32::from(self.ny)).map(move |k| (c, r + self.dy * k));
        std::iter::once((c, r)).chain(h).chain(v)
    }
}

/// Cells lent by an arm reaching `r` px from the stroke's centre, which
/// stands `own` px from the corner cell's edge on the arm's side.
fn lent(r: f32, own: f32, cell: f32) -> u16 {
    ((r - own) / cell).ceil().max(1.0) as u16
}

/// The corners of `cells` that can round at [`radius`], in cell order.
pub(crate) fn find(cells: &[CellView], cell_w: f32, cell_h: f32) -> Vec<Corner> {
    if !cells.iter().any(|c| arms(c.c).is_some()) {
        return Vec::new();
    }
    let at = Chars::of(cells);
    let r = radius(cell_w, cell_h);
    let (t, lx, ly) = stroke(cell_w, cell_h);
    let (cw, ch) = (cell_w.round(), cell_h.round());
    // Two corners on one rule round only if their arcs cannot meet: their
    // stroke centres at least two radii and a pixel apart. A one-row card's
    // top and bottom corners share the `│` between them, and that is fine —
    // each arc is done well inside its half of it.
    let crowded = |col: i32, row: i32, (dx, dy): (i32, i32), cell: f32| {
        (1..)
            .take_while(|&k| k as f32 * cell < 2.0 * r + 1.0)
            .any(|k| at.get(col + dx * k, row + dy * k).and_then(arms).is_some())
    };
    let mut out = Vec::new();
    for cell in cells {
        let Some((dx, dy)) = arms(cell.c) else {
            continue;
        };
        // From the stroke's centre to the corner cell's edge on each arm.
        let own_x = if dx > 0 {
            cw - lx - t / 2.0
        } else {
            lx + t / 2.0
        };
        let own_y = if dy > 0 {
            ch - ly - t / 2.0
        } else {
            ly + t / 2.0
        };
        let corner = Corner {
            col: cell.col,
            row: cell.row,
            dx,
            dy,
            nx: lent(r, own_x, cell_w),
            ny: lent(r, own_y, cell_h),
            fg_h: cell.fg,
            fg_v: cell.fg,
        };
        let (c, row) = (i32::from(cell.col), i32::from(cell.row));
        let plain = (1..=i32::from(corner.nx)).all(|k| at.get(c + dx * k, row) == Some(RULE_H))
            && (1..=i32::from(corner.ny)).all(|k| at.get(c, row + dy * k) == Some(RULE_V));
        if !plain || crowded(c, row, (dx, 0), cell_w) || crowded(c, row, (0, dy), cell_h) {
            continue;
        }
        let fg = |col: i32, row: i32| at.cell(col, row).map_or(cell.fg, |(_, fg)| fg);
        out.push(Corner {
            fg_h: fg(c + dx * i32::from(corner.nx), row),
            fg_v: fg(c, row + dy * i32::from(corner.ny)),
            ..corner
        });
    }
    out
}

/// `pane` with every corner in `corners` — glyph and borrowed arms — blanked,
/// so the arcs drawn in their place stand alone.
pub(crate) fn blanked(pane: &PaneScene, corners: &[Corner]) -> PaneScene {
    let gone: HashSet<(i32, i32)> = corners.iter().flat_map(Corner::cells).collect();
    let mut out = pane.clone();
    for c in out
        .cells
        .iter_mut()
        .filter(|c| gone.contains(&(i32::from(c.col), i32::from(c.row))))
    {
        c.c = ' ';
    }
    out
}

/// Where the rule glyphs of one drawn scene land: its buffers, the stretch
/// that split it into them and the cell size they were laid out on.
pub(crate) struct Drawn<'a> {
    pub pane: &'a PaneScene,
    pub split: Option<&'a Split>,
    pub buffers: &'a [PaneBuffer],
    pub cell_w: f32,
    pub cell_h: f32,
}

/// The pixel origin of global cell `(col, row)` as glyphon places a box
/// glyph laid into it — read from the buffer that draws it.
///
/// Top: `trunc(origin) + ceil(line_top)` — glyphon puts a glyph at its line's
/// rounded baseline less the synthesized placement, which is that baseline
/// less the line's top, truncated. Left: the cell's shaped x at the origin,
/// snapped by cosmic-text's subpixel bins (a fraction past 7/8 rounds up).
fn cell_at(d: &Drawn<'_>, col: i32, row: i32) -> Option<(f32, f32)> {
    let (col, row) = (u16::try_from(col).ok()?, u16::try_from(row).ok()?);
    let (idx, lcol, lrow) = match d.split {
        Some(s) => crate::stretch::locate(d.pane, s, col, row),
        None => (0, col, row),
    };
    let (buf, ox, oy, _, _) = d.buffers.get(idx)?;
    let run = buf.layout_runs().find(|r| r.line_i == usize::from(lrow));
    let top = match &run {
        Some(r) => {
            let base = r.line_y.round();
            base + oy.trunc() - (base - r.line_top).trunc()
        }
        None => oy.trunc() + (f32::from(lrow) * d.cell_h).ceil(),
    };
    let glyph = run.as_ref().and_then(|r| {
        r.glyphs
            .iter()
            .find(|g| (g.x / d.cell_w).round() as i64 == i64::from(lcol))
    });
    let left = match glyph {
        Some(g) => g.physical((*ox, *oy), 1.0).x as f32,
        None => {
            let x = ox + f32::from(lcol) * d.cell_w;
            let whole = x.trunc();
            if x - whole >= 0.875 {
                whole + 1.0
            } else {
                whole
            }
        }
    };
    Some((left, top))
}

/// The arcs for `corners`, placed on the glyphs of `d` and ready for the
/// round-border pass.
pub(crate) fn arcs(d: &Drawn<'_>, corners: &[Corner], srgb: bool) -> Vec<Border> {
    let r = radius(d.cell_w, d.cell_h);
    let (t, lx, ly) = stroke(d.cell_w, d.cell_h);
    let (cw, ch) = (d.cell_w.round(), d.cell_h.round());
    if corners.is_empty() {
        return Vec::new();
    }
    let chars = Chars::of(&d.pane.cells);
    let char_at = |col: i32, row: i32| chars.get(col, row);
    // Far enough that the shape's other three corners never reach the clip.
    let big = 4.0 * (r + cw + ch);
    let mut out = Vec::new();
    for k in corners {
        let (c, row) = (i32::from(k.col), i32::from(k.row));
        let Some((cx, cy)) = cell_at(d, c, row) else {
            continue;
        };
        // The cells just past the patch, where the untouched rules resume.
        let (hc, vr) = (
            c + k.dx * (i32::from(k.nx) + 1),
            row + k.dy * (i32::from(k.ny) + 1),
        );
        let h_rule = char_at(hc, row) == Some(RULE_H);
        let v_rule = char_at(c, vr) == Some(RULE_V);
        let h_next = cell_at(d, hc, row);
        let v_next = cell_at(d, c, vr);
        // The bands the tails continue: the neighbouring rule's own, when it
        // is one; the corner's otherwise.
        let by = match (h_rule, h_next) {
            (true, Some((_, top))) => top + ly,
            _ => cy + ly,
        };
        let bx = match (v_rule, v_next) {
            (true, Some((left, _))) => left + lx,
            _ => cx + lx,
        };
        // The clip: the corner cell, out to where the next cell begins along
        // each arm — a pixel into it when that cell is a rule, so the join
        // has no seam to show.
        let (ox, oy) = (f32::from(u8::from(h_rule)), f32::from(u8::from(v_rule)));
        let (x0, x1) = if k.dx > 0 {
            let end = h_next.map_or(cx + cw * (f32::from(k.nx) + 1.0), |n| n.0);
            (cx - 1.0, end + ox)
        } else {
            let end = h_next.map_or(cx - cw * f32::from(k.nx), |n| n.0 + cw);
            (end - ox, cx + cw + 1.0)
        };
        let (y0, y1) = if k.dy > 0 {
            let end = v_next.map_or(cy + ch * (f32::from(k.ny) + 1.0), |n| n.1);
            (cy - 1.0, end + oy)
        } else {
            let end = v_next.map_or(cy - ch * f32::from(k.ny), |n| n.1 + ch);
            (end - oy, cy + ch + 1.0)
        };
        // The ring's shape: its outer edge on the bands' outer edges, the
        // far corners `big` away.
        let (sx0, sx1) = if k.dx > 0 {
            (bx, bx + big)
        } else {
            (bx + t - big, bx + t)
        };
        let (sy0, sy1) = if k.dy > 0 {
            (by, by + big)
        } else {
            (by + t - big, by + t)
        };
        out.push(Border {
            x: sx0,
            y: sy0,
            w: sx1 - sx0,
            h: sy1 - sy0,
            radius: r + t / 2.0,
            thickness: t,
            color: crate::color::target_rgba(k.fg_v, 1.0, srgb),
            color_h: Some(crate::color::target_rgba(k.fg_h, 1.0, srgb)),
            clip: Some([x0, y0, x1, y1]),
        });
    }
    out
}

#[cfg(test)]
#[path = "corners_tests.rs"]
mod tests;
