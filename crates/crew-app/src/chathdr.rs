//! The crew pane's header row: a title on the left and a right-aligned live
//! status — a connection dot, the message count, and an animated "thinking"
//! spinner while a reply is pending. Rendered as row 0 of the pane, with the
//! message body laid out below it.
use crew_render::CellView;

/// ASCII spinner frames for the "thinking" indicator (Nerd-Font-independent).
const SPINNER: [char; 4] = ['|', '/', '-', '\\'];

/// Append `s` at `(row, col..)` in `fg`; returns the next free column.
fn push(
    cells: &mut Vec<CellView>,
    row: u16,
    col: u16,
    s: &str,
    fg: (u8, u8, u8),
    bold: bool,
) -> u16 {
    let bg = crew_theme::theme().page_bg;
    // Width-aware (see `chatwidth`): agent labels can carry wide glyphs.
    crate::chatwidth::place_row(col, u16::MAX, s.chars().map(|c| (c, fg)), |x, c, fg| {
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

/// The right-aligned status segments as `(text, colour)`, in left-to-right order.
/// Joined with two-space gaps. While an agent is active the spinner names it and
/// counts the elapsed seconds (`| coder · 12s`, in the agent's roster colour);
/// otherwise a plain `thinking` spinner appears while a send is unanswered.
fn status_segments(
    connected: bool,
    msg_count: usize,
    awaiting: bool,
    active: Option<(&str, u64, (u8, u8, u8))>,
    tokens: u64,
    turns: u64,
) -> Vec<(String, (u8, u8, u8))> {
    let t = crew_theme::theme();
    let mut segs = Vec::new();
    let f = (crate::anim::now_ms() / 120) as usize % SPINNER.len();
    if let Some((label, secs, color)) = active {
        segs.push((format!("{} {label} \u{00b7} {secs}s", SPINNER[f]), color));
    } else if awaiting {
        segs.push((format!("{} thinking", SPINNER[f]), crate::palette::accent()));
    }
    if turns > 0 {
        let plural = if turns == 1 { "" } else { "s" };
        segs.push((format!("{turns} turn{plural}"), t.text_muted));
    }
    if tokens > 0 {
        segs.push((format!("~{} tok", fmt_tokens(tokens)), t.text_muted));
    }
    let plural = if msg_count == 1 { "" } else { "s" };
    segs.push((format!("{msg_count} msg{plural}"), t.text_muted));
    let (dot, dot_c) = if connected {
        ('\u{25cf}', t.activity) // ● connected
    } else {
        ('\u{25cb}', t.dim) // ○ connecting
    };
    segs.push((dot.to_string(), dot_c));
    segs
}

/// A compact token count: `950`, then `9.5k` from a thousand up.
fn fmt_tokens(tokens: u64) -> String {
    if tokens < 1_000 {
        tokens.to_string()
    } else {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    }
}

/// Build the single-row header for a `cols`-wide crew pane.
/// `totals` is the session's `(approx tokens, completed turns)`.
pub(crate) fn header_cells(
    cols: u16,
    channel: &str,
    connected: bool,
    msg_count: usize,
    awaiting: bool,
    active: Option<(&str, u64, (u8, u8, u8))>,
    totals: (u64, u64),
) -> Vec<CellView> {
    let (tokens, turns) = totals;
    if cols == 0 {
        return Vec::new();
    }
    let mut cells = Vec::new();

    // Title, left-aligned (truncated by the right-side status if space is tight).
    let title = if channel.is_empty() {
        "crew".to_string()
    } else {
        format!("crew \u{00b7} {channel}") // crew · <channel>
    };

    // Right-aligned status, laid out from the right edge.
    let segs = status_segments(connected, msg_count, awaiting, active, tokens, turns);
    let status_w: usize = segs
        .iter()
        .map(|(s, _)| crate::chatwidth::str_w(s))
        .sum::<usize>()
        + segs.len().saturating_sub(1) * 2;
    let mut x = cols.saturating_sub(status_w as u16);
    for (i, (s, c)) in segs.iter().enumerate() {
        if i > 0 {
            x += 2; // two-space gap between segments
        }
        if x < cols {
            x = push(&mut cells, 0, x, s, *c, false);
        }
    }

    // Title only up to where the status begins (measured in display columns).
    let title_room = cols.saturating_sub(status_w as u16 + 1) as usize;
    let full: Vec<char> = title.chars().collect();
    let end = crate::chatwidth::fit_end(&full, 0, title_room);
    let title: String = full[..end].iter().collect();
    push(&mut cells, 0, 0, &title, crate::palette::accent(), true);

    cells
}

#[cfg(test)]
#[path = "chathdr_tests.rs"]
mod tests;
