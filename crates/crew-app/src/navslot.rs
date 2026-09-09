//! The nav's variable slot: the glance cards (SERVING, WAITING ON YOU) or
//! the LOG tail, drawn at the rows `navlayout` gave them. Split from
//! `statspane` so the stack there stays a stack — one call per section.
use crew_render::{CellView, Paint};

use crate::applog::LogEntry;
use crate::navglance::Glance;
use crate::navlayout::{NavLayout, Tail};

/// How the slot is filled this frame, for the layout.
pub(crate) fn tail(glance: Option<&Glance>, log_len: usize) -> Tail {
    match glance {
        Some(g) => Tail::Glance(g.waiting.len()),
        None => Tail::Log(log_len),
    }
}

/// The slot's cells, already shifted to their rows.
pub(crate) fn slot_cells(
    glance: Option<&Glance>,
    log: &[LogEntry],
    log_back: usize,
    l: &NavLayout,
    cols: u16,
) -> Vec<CellView> {
    let mut out = Vec::new();
    let Some(g) = glance else {
        if l.log_lines > 0 {
            for mut c in crate::navlog::log_cells(log, cols, l.log_lines, log_back) {
                c.row += l.log_top;
                out.push(c);
            }
        }
        return out;
    };
    if l.serving_rows > 0 {
        let (mut cells, meters) = crate::navserving::serving_cells(&g.serving, cols);
        // The paint pass draws the meters (`slot_paint`); the glyphs that
        // reserve their columns must not show through them.
        let _ = crate::summarymeter::draw_meters(&mut cells, &meters, 1.0);
        for mut c in cells {
            c.row += l.serving_top;
            out.push(c);
        }
    }
    for mut c in crate::navwaiting::waiting_cells(&g.waiting, cols, l.waiting_lines) {
        c.row += l.waiting_top;
        out.push(c);
    }
    out
}

/// The SERVING card's two meters, drawn as the footer draws its own.
pub(crate) fn slot_paint(g: &Glance, l: &NavLayout, cols: u16, aspect: f32) -> Vec<Paint> {
    if l.serving_rows == 0 {
        return Vec::new();
    }
    let (mut cells, meters) = crate::navserving::serving_cells(&g.serving, cols);
    for c in &mut cells {
        c.row += l.serving_top;
    }
    crate::summarymeter::draw_meters(&mut cells, &meters, aspect)
}
