//! Sidebar load section: a `LOAD` divider above the 1/5/15-minute system load
//! average, coloured by load-per-core (green / amber / red). Complements the
//! instantaneous SYSTEM gauges with a sense of sustained pressure.
use crew_render::CellView;

use crate::palette::accent;
/// The two warning colours, darkened until the page can carry them. They
/// shipped as the flat constants `(230, 180, 90)` and `(230, 90, 90)`, which
/// on a light theme read at 1.7 and 3.2 — a warning colour at 1.7 is a warning
/// nobody receives.
fn amber() -> (u8, u8, u8) {
    crew_theme::readable::warn(crew_theme::theme())
}

fn red() -> (u8, u8, u8) {
    crew_theme::readable::danger(crew_theme::theme())
}

/// Current `(one, five, fifteen)`-minute load averages (0.0 where unsupported).
pub fn load_avg() -> (f64, f64, f64) {
    let l = sysinfo::System::load_average();
    (l.one, l.five, l.fifteen)
}

/// Logical-core count, used to scale the load colour (never zero).
pub fn cores() -> f64 {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .max(1) as f64
}

/// Colour for a 1-minute load over `cores`: green when comfortably under one
/// task per core, amber approaching saturation, red once oversubscribed.
fn load_color(one: f64, cores: f64) -> (u8, u8, u8) {
    let per_core = one / cores;
    if per_core < 0.7 {
        accent()
    } else if per_core < 1.0 {
        amber()
    } else {
        red()
    }
}

/// Render the load section: a `LOAD 1·5·15m` rule on row 0 and as many of the
/// three averages as the nav is wide enough to draw whole on row 1, coloured
/// by the 1-minute load relative to `cores`.
pub fn load_cells(one: f64, five: f64, fifteen: f64, cores: f64, cols: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let fg = load_color(one, cores);
    let past = crew_theme::readable::secondary(fg, t.page_bg);
    // Longest first. A narrow nav gives up the oldest average whole rather
    // than cutting the last one in half — `3.` used to be what the docked nav
    // showed at the narrow end of the resize range, and `3.` is not a smaller
    // reading than `3.64`, it is a wrong one.
    let ladder = [
        format!("{one:.2}  {five:.2}  {fifteen:.2}"),
        format!("{one:.2} {five:.2} {fifteen:.2}"),
        format!("{one:.2} {five:.2}"),
        format!("{one:.2}"),
    ];
    let refs: Vec<&str> = ladder.iter().map(String::as_str).collect();
    let shown = crate::navtext::fit(&refs, cols);
    // The rule's key names exactly the averages that survived: a key claiming
    // three when two are drawn is worse than no key at all.
    let key = match shown.split_whitespace().count() {
        0..=1 => "1m",
        2 => "1·5m",
        _ => "1·5·15m",
    };
    let mut out = crate::boxdraw::section_header_key(
        "LOAD",
        key,
        cols,
        t.border_normal,
        accent(),
        t.dim,
        t.page_bg,
    );
    // One colour, three ranks: the 1-minute figure is the only one that says
    // anything about *now*, so it keeps the load colour at full strength and
    // the history steps back. Separate hues would say they measured different
    // things; `text_muted` would throw away the warning they share.
    // The separator the fitted rung chose, so the coloured runs land exactly
    // where the string put them rather than on a spacing rule of their own.
    let sep = if shown.contains("  ") { 2 } else { 1 };
    let mut col = crate::navtext::INDENT;
    for (i, word) in shown.split_whitespace().enumerate() {
        crate::navtext::put_at(
            &mut out,
            word,
            col,
            1,
            cols.saturating_sub(1),
            if i == 0 { fg } else { past },
        );
        col += word.chars().count() as u16 + sep;
    }
    out
}

#[cfg(test)]
#[path = "load_tests.rs"]
mod tests;
