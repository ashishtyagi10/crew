//! The agent smith pane's header row: a right-aligned live status only — a
//! connection dot and an animated "thinking" spinner while a reply is pending.
//! The pane's fieldset legend already names it, so the old in-pane
//! `agent smith · <channel>` title was pure repetition and is gone. Rendered
//! as row 0 of the pane, with the message body laid out below it.
use crate::chathdrsegs::{status_segments, Seg};
use crate::shimmer::Color;
use crew_render::CellView;

/// Display width of a segment (agent labels can carry wide glyphs).
fn seg_w(s: &Seg) -> usize {
    s.iter().map(|(c, _)| crate::chatwidth::char_w(*c)).sum()
}

/// Append `s` on row 0 at `col..`, clipped to `max_col`; returns the next
/// free column. Width-aware through `chatwidth::place_row`.
fn push(cells: &mut Vec<CellView>, col: u16, max_col: u16, s: &Seg) -> u16 {
    let bg = crew_theme::theme().page_bg;
    crate::chatwidth::place_row(col, max_col, s.iter().copied(), |x, c, fg| {
        cells.push(CellView {
            col: x,
            row: 0,
            c,
            fg,
            bg,
            ..Default::default()
        });
    })
}

/// A compact token count: `950`, then `9.5k` from a thousand up.
///
/// Past a million it climbs to `M` (then `G`): a long session read
/// `4123.5k in` on the footer while `/usage` beside it said `4.1M in`.
pub(crate) fn fmt_tokens(tokens: u64) -> String {
    let t = tokens as f64;
    match tokens {
        0..=999 => tokens.to_string(),
        1_000..=999_949 => format!("{:.1}k", t / 1e3),
        999_950..=999_949_999 => format!("{:.1}M", t / 1e6),
        _ => format!("{:.1}G", t / 1e9),
    }
}

/// Gap before segment `i` of `segs`: none before the first, a single space
/// before the trailing connection dot and before a segment that opens with
/// its own `·` (the hint, the compact chip — two spaces there read
/// `thinking  · esc interrupts`, where every other reading in crew is
/// `a · b`), two between the rest.
fn gap(i: usize, segs: &[Seg]) -> usize {
    let dotted = segs[i].first().is_some_and(|&(c, _)| c == '\u{00b7}');
    match i {
        0 => 0,
        _ if i + 1 == segs.len() || dotted => 1,
        _ => 2,
    }
}

/// The columns a run of segments takes, gaps included. Shared by the width
/// probe and the real layout so both agree on the same gap rule.
fn segs_width(segs: &[Seg]) -> usize {
    segs.iter()
        .enumerate()
        .map(|(i, s)| gap(i, segs) + seg_w(s))
        .sum()
}

/// Build the single-row header for a `cols`-wide agent smith pane — liveness
/// only (the fieldset legend carries the identity); session stats live in the
/// below-input summary footer. Read at the shared animation clock; see
/// [`header_cells_at`] for the clock as a parameter.
pub(crate) fn header_cells(
    cols: u16,
    _channel: &str,
    connected: bool,
    awaiting: bool,
    active: Option<(&str, u64, Color)>,
    compact: bool,
    tools: usize,
) -> Vec<CellView> {
    let now = crate::anim::now_ms();
    header_cells_at(cols, connected, awaiting, active, compact, tools, now)
}

/// [`header_cells`] at an explicit `now_ms` — the spinner frame, the
/// shimmer and the breath are all functions of it.
/// `compact` (Ctrl+O — `ChatPane::compact_view`) shows a muted "compact" chip;
/// it's the first thing dropped on a narrow pane, ahead of the busy hint.
pub(crate) fn header_cells_at(
    cols: u16,
    connected: bool,
    awaiting: bool,
    active: Option<(&str, u64, Color)>,
    compact: bool,
    tools: usize,
    now_ms: u64,
) -> Vec<CellView> {
    if cols == 0 {
        return Vec::new();
    }
    // Try with both the compact chip and the busy hint first. If it doesn't
    // fit, the compact chip is the first thing dropped (it's the less
    // essential of the two); if it still doesn't fit, the hint goes too —
    // everything else (spinner/active label, connection dot) renders exactly
    // as it would without either.
    let build =
        |chip, hint| status_segments(connected, awaiting, active, chip, hint, tools, now_ms);
    let mut segs = build(compact, true);
    if segs_width(&segs) > cols as usize {
        segs = build(false, true);
    }
    if segs_width(&segs) > cols as usize {
        segs = build(false, false);
    }
    // Right-aligned, laid out from the right edge.
    let mut cells = Vec::new();
    let mut x = cols.saturating_sub(segs_width(&segs) as u16);
    for (i, s) in segs.iter().enumerate() {
        x += gap(i, &segs) as u16;
        if x < cols {
            x = push(&mut cells, x, cols, s);
        }
    }
    cells
}

#[cfg(test)]
#[path = "chathdr_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chathdrglow_tests.rs"]
mod glow_tests;

#[cfg(test)]
#[path = "chathdrtokens_tests.rs"]
mod tokens_tests;
