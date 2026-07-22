//! The agent smith pane's header row: a right-aligned live status only — a
//! connection dot and an animated "thinking" spinner while a reply is pending.
//! The pane's fieldset legend already names it, so the old in-pane
//! `agent smith · <channel>` title was pure repetition and is gone. Rendered
//! as row 0 of the pane, with the message body laid out below it.
use crew_render::CellView;

/// ASCII spinner frames for the "thinking" indicator (Nerd-Font-independent).
const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

/// Append `s` at `(row, col..)` in `fg`, clipped to `max_col`; returns the
/// next free column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    max_col: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    // Width-aware (see `chatwidth`): agent labels can carry wide glyphs.
    crate::chatwidth::place_row(col, max_col, s.chars().map(|c| (c, fg)), |x, c, fg| {
        cells.push(CellView {
            col: x,
            row,
            c,
            fg,
            bg,
            bold,
            italic: false,
        });
    })
}

/// The muted hint appended to the status while the pane is busy — Esc
/// cancels the running turn instead of closing the pane (see the
/// esc-interrupt design doc). Its own segment, inserted right before the
/// connection dot, so [`header_cells`] can drop it first — before touching
/// anything else — when the pane is too narrow for the full status.
const INTERRUPT_HINT: &str = "\u{00b7} esc interrupts";

/// The muted chip shown while the transcript is in compact view (Ctrl+O —
/// see `ChatPane::compact_view`). Same segment family as [`INTERRUPT_HINT`]
/// (an optional, droppable `· label` suffix), but less essential than the
/// busy hint, so [`header_cells`] drops it first when the pane is too narrow
/// for the full status — before touching the esc-interrupts hint.
const COMPACT_CHIP: &str = "\u{00b7} compact";

/// The right-aligned status segments as `(text, colour)`, in left-to-right order.
/// While an agent is active the spinner names it and counts the elapsed
/// seconds (`| coder · 12s`, in the agent's roster colour); otherwise a plain
/// `thinking` spinner appears while a send is unanswered. The trailing
/// connection dot keeps the tighter single-space gap it always had. Session
/// stats (model, context, tokens) live in the below-input summary footer
/// (`chatsummary`) — the header is identity and liveness only, never a second
/// place the same numbers get repeated.
/// `hint`, when the pane is busy, adds the muted "esc interrupts" segment
/// just before the dot — callers drop it (`hint: false`) to reclaim width on
/// narrow panes. `compact`, when the transcript is in compact view, adds the
/// muted "compact" chip after the spinner (dropped first of the two —
/// see [`COMPACT_CHIP`] — via `compact: false`).
fn status_segments(
    connected: bool,
    awaiting: bool,
    active: Option<(&str, u64, (u8, u8, u8))>,
    compact: bool,
    hint: bool,
) -> Vec<(String, (u8, u8, u8))> {
    let t = crew_theme::theme();
    let mut segs = Vec::new();
    let f = (crate::anim::now_ms() / 120) as usize % SPINNER.len();
    if let Some((label, secs, color)) = active {
        segs.push((format!("{} {label} \u{00b7} {secs}s", SPINNER[f]), color));
    } else if awaiting {
        segs.push((format!("{} thinking", SPINNER[f]), crate::palette::accent()));
    }

    // Compact-view chip, width-permitting — appended after the spinner,
    // ahead of the busy hint (see `header_cells`: it's the first of the two
    // dropped on a narrow pane).
    if compact {
        segs.push((COMPACT_CHIP.to_string(), t.text_muted));
    }

    // Busy hint, width-permitting (see `header_cells`) — appended after the
    // counters (and the compact chip, if shown), before the connection dot.
    if awaiting && hint {
        segs.push((INTERRUPT_HINT.to_string(), t.text_muted));
    }

    let (dot, dot_c) = if connected {
        ('\u{25cf}', t.activity) // ● connected
    } else {
        ('\u{25cb}', t.dim) // ○ connecting
    };
    segs.push((dot.to_string(), dot_c));
    segs
}

/// A compact token count: `950`, then `9.5k` from a thousand up.
pub(crate) fn fmt_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        tokens.to_string()
    } else {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    }
}

/// Build the single-row header for a `cols`-wide agent smith pane — liveness
/// only (the fieldset legend carries the identity); session stats live in the
/// below-input summary footer.
/// `compact` (Ctrl+O — `ChatPane::compact_view`) shows a muted "compact" chip;
/// it's the first thing dropped on a narrow pane, ahead of the busy hint.
pub(crate) fn header_cells(
    cols: u16,
    _channel: &str,
    connected: bool,
    awaiting: bool,
    active: Option<(&str, u64, (u8, u8, u8))>,
    compact: bool,
) -> Vec<CellView> {
    if cols == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();

    // Right-aligned status, laid out from the right edge. Segments get the
    // usual two-space gap, except the trailing connection dot, which sits a
    // single space after the token meter. `segs_width` is shared by the
    // width probe below and the real layout, so both agree on the same gap
    // rule for whatever segment count they're given.
    let segs_width = |segs: &[(String, (u8, u8, u8))]| -> usize {
        let gap = |i: usize| -> u16 {
            if i == 0 {
                0
            } else if i == segs.len() - 1 {
                1 // tight gap before the trailing connection dot
            } else {
                2
            }
        };
        segs.iter()
            .map(|(s, _)| crate::chatwidth::str_w(s))
            .sum::<usize>()
            + (0..segs.len()).map(gap).sum::<u16>() as usize
    };
    // Try with both the compact chip and the busy hint first. If it doesn't
    // fit, the compact chip is the first thing dropped (it's the less
    // essential of the two); if it still doesn't fit, the hint goes too —
    // everything else (spinner/active label, token meter, connection dot)
    // renders exactly as it would without either.
    let mut segs = status_segments(connected, awaiting, active, compact, true);
    if segs_width(&segs) as u16 > cols {
        segs = status_segments(connected, awaiting, active, false, true);
    }
    if segs_width(&segs) as u16 > cols {
        segs = status_segments(connected, awaiting, active, false, false);
    }
    let gap = |i: usize| -> u16 {
        if i == 0 {
            0
        } else if i == segs.len() - 1 {
            1 // tight gap before the trailing connection dot
        } else {
            2
        }
    };
    let status_w: usize = segs_width(&segs);
    let mut x = cols.saturating_sub(status_w as u16);
    for (i, (s, c)) in segs.iter().enumerate() {
        x += gap(i);
        if x < cols {
            x = push(&mut cells, 0, x, cols, s, *c, false);
        }
    }

    cells
}

#[cfg(test)]
#[path = "chathdr_tests.rs"]
mod tests;
