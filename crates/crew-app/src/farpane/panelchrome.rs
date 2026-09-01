//! A `/far` panel's furniture: its legend, the scroll thumb, the divider
//! between the two sides, and how a size and a directory read.
//!
//! Split from [`super::render`] for the line cap, along the line between
//! drawing the listing and drawing what frames it.
use super::Panel;
use crate::palette::accent_color;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

/// Directory entries, from the theme's own cyan slot rather than one fixed
/// blue-cyan. All sixteen presets tune that slot, and the single-phosphor
/// tubes tune it to a colour the tube can actually draw — a `(120, 200, 255)`
/// folder on a green screen is a colour that phosphor does not have.
pub(crate) fn dir_color() -> Color {
    let t = crew_theme::theme();
    let (r, g, b) =
        crew_theme::readable::against(t.ansi[6], t.page_bg, crew_theme::contrast::text_floor());
    Color::Rgb(r, g, b)
}

/// Halve `area` with a one-column overlap, so the panels share their middle
/// border instead of drawing `││` (which reads as a wide gap on screen).
pub(crate) fn split_panels(area: Rect) -> (Rect, Rect) {
    let lw = area.width / 2 + 1;
    (
        Rect::new(area.x, area.y, lw, area.height),
        Rect::new(area.x + lw - 1, area.y, area.width - lw + 1, area.height),
    )
}

/// Join the shared border column into the panel frames: `┬` at the top, `┴`
/// at the bottom, accent-coloured — the divider always touches the active
/// panel, whichever side it is.
pub(crate) fn merge_divider(buf: &mut Buffer, area: Rect, x: u16) {
    for y in area.y..area.y + area.height {
        let sym = if y == area.y {
            "\u{252c}" // ┬
        } else if y == area.y + area.height - 1 {
            "\u{2534}" // ┴
        } else {
            "\u{2502}" // │
        };
        if let Some(cell) = buf.cell_mut((x, y)) {
            cell.set_symbol(sym);
            cell.set_fg(accent_color());
        }
    }
}

/// Paint the proportional scroll thumb over `panel`'s right border while its
/// listing overflows. Called from `render` AFTER both panels and the divider
/// are drawn, since the left panel's border is the shared middle column.
pub(crate) fn scroll_thumb(buf: &mut Buffer, area: Rect, panel: &Panel, active: bool) {
    let inner_h = area.height.saturating_sub(2) as usize; // minus top/bottom border
    let start = panel
        .sel
        .saturating_sub(inner_h.saturating_sub(1))
        .min(panel.sel);
    let Some((top, len)) = crate::chatscroll::thumb(panel.entries.len(), inner_h, start) else {
        return;
    };
    let edge = if active {
        accent_color()
    } else {
        let t = crew_theme::theme();
        Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2)
    };
    let x = area.x + area.width - 1;
    for i in 0..len {
        if let Some(cell) = buf.cell_mut((x, area.y + 1 + (top + i) as u16)) {
            cell.set_symbol("\u{2588}"); // █
            cell.set_fg(edge);
        }
    }
}

/// `bytes` in compact Far-style units: `427 B`, `1.2K`, `34M`, `2.1G` — one
/// decimal below 10, none above, binary (1024) steps.
pub(crate) fn fmt_size(bytes: u64) -> String {
    const UNITS: [char; 4] = ['K', 'M', 'G', 'T'];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut v = bytes as f64 / 1024.0;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < UNITS.len() {
        v /= 1024.0;
        i += 1;
    }
    if v < 10.0 {
        format!("{v:.1}{}", UNITS[i])
    } else {
        format!("{v:.0}{}", UNITS[i])
    }
}

/// `" /path · N · size "` — `N` is the panel's entry count and `size` its
/// total byte size (via `fmt_size`). A directory with zero entries shows
/// `· empty` instead of the (always-zero, redundant) `· 0 · 0 B` — a plain
/// word reads faster than two zeros. The suffix stays intact whenever
/// there's room for it at all; the path truncates from the left (keeping the
/// tail) to fit `width`, same as before the count/size were added.
///
/// `width` is the panel's whole width, borders included — a ratatui block
/// title only owns what is between them, and it was being fitted to all of
/// it. So the rightmost characters were clipped by the block with nothing to
/// show for it: at a tile width the header read `· 3.3` and the size lost its
/// unit. Three columns come off the top: the two border cells, plus one of
/// rule after the title, which is the breath every other card in crew keeps
/// (`boxdraw::title_budget` takes six for the same reason).
/// Columns the path keeps for itself before the count and size are worth
/// showing at all.
pub(crate) const MIN_PATH: usize = 6;

pub(crate) fn legend(display: &str, count: usize, total: u64, width: u16) -> String {
    let suffix = if count == 0 {
        " \u{00b7} empty ".to_string()
    } else {
        format!(" \u{00b7} {count} \u{00b7} {} ", fmt_size(total))
    };
    let usable = (width as usize).saturating_sub(3);
    // A panel too narrow for the count and the size keeps the thing you
    // actually navigate by. The suffix is dropped rather than clipped — the
    // old `max == 0` branch returned the whole title anyway and let the block
    // cut it, which is how `· 3.3` came to be a thing the header said.
    let suffix = match suffix.chars().count() + MIN_PATH <= usable {
        true => suffix,
        false => " ".to_string(),
    };
    let max = usable.saturating_sub(1 + suffix.chars().count());
    if max == 0 {
        return String::new();
    }
    if display.chars().count() <= max {
        return format!(" {display}{suffix}");
    }
    let tail: String = display
        .chars()
        .rev()
        .take(max.saturating_sub(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!(" …{tail}{suffix}")
}
