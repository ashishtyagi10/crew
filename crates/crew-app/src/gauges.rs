//! Pure rendering of the system-stats sidebar section: a header + spaced gauges.
use crew_render::CellView;

use crate::boxdraw;
use crate::palette::accent;
use crate::stats::Stats;

const HEADER: &str = "SYSTEM";

/// Empty-track colour: the theme's recessed border shade, so the track sits
/// back like an unfocused card edge — and stays in-palette on the monochrome
/// CRT phosphor themes instead of a fixed grey.
pub(crate) fn track_color() -> (u8, u8, u8) {
    crew_theme::theme().border_normal
}

/// Bar (and ring) colour by load: accent when low, the theme's status amber past 70%,
/// its bright-red ANSI slot past 90% — every tier drawn from the active
/// palette so a phosphor theme's gauges glow in that phosphor's hues.
pub(crate) fn fill_color(frac: f32) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    // The bands come from `shapecues::Tier` so the colour and the shape cue
    // beside it can never disagree about which band a reading is in.
    match crate::shapecues::Tier::of(frac) {
        crate::shapecues::Tier::Nominal => accent(),
        crate::shapecues::Tier::Warn => t.status_fg,
        crate::shapecues::Tier::Critical => t.ansi[9],
    }
}

/// One gauge row laid out within `cols`: `label | space | bar | NNN%`.
fn gauge_cells(label: &str, frac: f32, row: u16, cols: u16) -> Vec<CellView> {
    if cols == 0 {
        return Vec::new();
    }
    let cols = cols as usize;
    let pct = (frac.clamp(0.0, 1.0) * 100.0).round() as u16;
    let pct_str = format!("{pct:>3}%");
    let pct_len = pct_str.len();

    let t = crew_theme::theme();
    let label_chars: Vec<char> = label.chars().collect();
    let label_len = label_chars.len();
    let mut cells: Vec<CellView> = Vec::with_capacity(cols);

    for (i, &c) in label_chars.iter().enumerate() {
        if cells.len() >= cols {
            break;
        }
        cells.push(cell(i as u16, row, c, t.ink, t.page_bg));
    }
    // The tier mark rides in the label's trailing space, in the fill's own
    // colour: the band is said twice for anyone who needs it and costs no
    // column, since that space was always there (see `shapecues`).
    let mark = crate::shapecues::Tier::of(frac).mark();
    if cells.len() < cols {
        let (c, fg) = match mark {
            Some(m) => (m, fill_color(frac)),
            None => (' ', t.ink),
        };
        cells.push(cell(label_len as u16, row, c, fg, t.page_bg));
    }

    let used = cells.len();
    let bar_width = cols.saturating_sub(label_len + 1 + pct_len);
    let filled = (frac.clamp(0.0, 1.0) * bar_width as f32).round() as usize;
    let fill = fill_color(frac);
    for i in 0..bar_width {
        if cells.len() >= cols {
            break;
        }
        let (c, fg) = if i < filled {
            ('█', fill)
        } else {
            ('░', track_color())
        };
        cells.push(cell((used + i) as u16, row, c, fg, t.page_bg));
    }

    let pct_start = cols.saturating_sub(pct_len);
    for (i, c) in pct_str.chars().enumerate() {
        let col = pct_start + i;
        if col >= cols {
            break;
        }
        if col < cells.len() {
            cells[col] = cell(col as u16, row, c, t.ink, t.page_bg);
        } else {
            cells.push(cell(col as u16, row, c, t.ink, t.page_bg));
        }
    }
    cells
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        bold: false,
        italic: false,
        ..Default::default()
    }
}

/// Render the stats section: a `SYSTEM` rule on row 0 (fieldset-legend style)
/// with the three readings under it — as instrument dials when the nav has
/// the columns for them ([`crate::sysdials`], which draws the faces and
/// leaves their text here), and as the labelled bars below when it does not.
/// Sidebar sections stack as their own dividers below.
pub(crate) fn render_stats(stats: Stats, cols: u16, rows: u16, peak: Option<u64>) -> Vec<CellView> {
    let mut out = Vec::new();
    if cols < 8 || rows < 4 {
        return out;
    }
    let t = crew_theme::theme();
    // The CPU curve under the gauges is scaled to its own rolling peak, not to
    // 0-100 — that is what lets a machine idling under 10% draw a shape at
    // all. A chart with a moving ceiling and no ceiling written down is a
    // chart you cannot read a number off, so the rule carries it.
    let key = peak.map(|p| format!("peak {p}%")).unwrap_or_default();
    out.extend(boxdraw::section_header_key(
        HEADER,
        &key,
        cols,
        t.border_normal,
        accent(),
        t.dim,
        t.page_bg,
    ));

    // Wide enough, the three readings are drawn as dials (see
    // `crate::sysdials`); this section then contributes only their text.
    if crate::sysdials::fits(cols) {
        out.extend(crate::sysdials::NAV.cells(stats, cols, 1));
        return out;
    }

    // Content indented to align under the section legend (col 3).
    let cstart = 3u16;
    let inner = cols.saturating_sub(cstart + 1);

    let gauges = [
        ("CPU ", stats.cpu),
        ("MEM ", stats.mem),
        ("DISK", stats.disk),
    ];
    for (i, (label, frac)) in gauges.into_iter().enumerate() {
        let row = 1 + i as u16;
        if row >= rows {
            break;
        }
        for mut g in gauge_cells(label, frac, 0, inner) {
            g.col += cstart;
            g.row = row;
            out.push(g);
        }
    }
    out
}

#[cfg(test)]
#[path = "gauges_tests.rs"]
mod tests;
