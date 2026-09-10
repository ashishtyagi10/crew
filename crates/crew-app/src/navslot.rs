//! The nav's variable slot: the glance cards (WEATHER, SERVING, WAITING ON YOU) or
//! the LOG tail, drawn at the rows `navlayout` gave them. Split from
//! `statspane` so the stack there stays a stack — one call per section.
use crew_render::{CellView, Paint};

use crate::applog::LogEntry;
use crate::navglance::Glance;
use crate::navlayout::{NavLayout, Tail};

/// How the slot is filled this frame, for the layout.
pub(crate) fn tail(glance: Option<&Glance>, log_len: usize) -> Tail {
    match glance {
        Some(g) => Tail::Glance {
            waiting: g.waiting.len(),
            weather: crate::navweathercard::block(&g.weather),
        },
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
    if l.weather_rows > 0 {
        for mut c in crate::navweathercard::state_cells(&g.weather, cols) {
            c.row += l.weather_top;
            out.push(c);
        }
    }
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

/// The WEATHER card's curve and the SERVING card's two meters, drawn as
/// the footer draws its own.
pub(crate) fn slot_paint(g: &Glance, l: &NavLayout, cols: u16, aspect: f32) -> Vec<Paint> {
    let mut out = match &g.weather {
        crate::navweather::State::Found(w) if l.weather_rows > 0 => {
            crate::navweathercurve::weather_paint(w, l.weather_top, cols, aspect)
        }
        _ => Vec::new(),
    };
    if l.serving_rows == 0 {
        return out;
    }
    let (mut cells, meters) = crate::navserving::serving_cells(&g.serving, cols);
    for c in &mut cells {
        c.row += l.serving_top;
    }
    out.extend(crate::summarymeter::draw_meters(
        &mut cells, &meters, aspect,
    ));
    out
}
