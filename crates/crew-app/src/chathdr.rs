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
    for (i, ch) in s.chars().enumerate() {
        cells.push(CellView {
            col: col + i as u16,
            row,
            c: ch,
            fg,
            bg,
            bold,
            italic: false,
        });
    }
    col + s.chars().count() as u16
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
) -> Vec<(String, (u8, u8, u8))> {
    let t = crew_theme::theme();
    let mut segs = Vec::new();
    let f = (crate::anim::now_ms() / 120) as usize % SPINNER.len();
    if let Some((label, secs, color)) = active {
        segs.push((format!("{} {label} \u{00b7} {secs}s", SPINNER[f]), color));
    } else if awaiting {
        segs.push((format!("{} thinking", SPINNER[f]), crate::palette::accent()));
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
pub(crate) fn header_cells(
    cols: u16,
    channel: &str,
    connected: bool,
    msg_count: usize,
    awaiting: bool,
    active: Option<(&str, u64, (u8, u8, u8))>,
    tokens: u64,
) -> Vec<CellView> {
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
    let segs = status_segments(connected, msg_count, awaiting, active, tokens);
    let status_w: usize = segs.iter().map(|(s, _)| s.chars().count()).sum::<usize>()
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

    // Title only up to where the status begins, so they never overlap.
    let title_room = cols.saturating_sub(status_w as u16 + 1) as usize;
    let title: String = title.chars().take(title_room).collect();
    push(&mut cells, 0, 0, &title, crate::palette::accent(), true);

    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(cells: &[CellView], row: u16) -> String {
        let mut v: Vec<(u16, char)> = cells
            .iter()
            .filter(|c| c.row == row)
            .map(|c| (c.col, c.c))
            .collect();
        v.sort_unstable();
        v.into_iter().map(|(_, c)| c).collect()
    }

    #[test]
    fn header_shows_title_channel_and_count() {
        let cells = header_cells(60, "general", true, 3, false, None, 0);
        let line = text(&cells, 0);
        assert!(line.contains("crew"), "title missing: {line}");
        assert!(line.contains("general"), "channel missing: {line}");
        assert!(line.contains("3 msgs"), "count missing: {line}");
        assert!(line.contains('\u{25cf}'), "connected dot missing: {line}");
    }

    #[test]
    fn singular_message_and_connecting_dot() {
        let line = text(&header_cells(60, "", false, 1, false, None, 0), 0);
        assert!(line.contains("1 msg") && !line.contains("1 msgs"));
        assert!(line.contains('\u{25cb}'), "connecting dot missing: {line}");
    }

    #[test]
    fn awaiting_shows_thinking_spinner() {
        let line = text(&header_cells(60, "c", true, 0, true, None, 0), 0);
        assert!(line.contains("thinking"), "spinner label missing: {line}");
    }

    #[test]
    fn active_agent_shows_name_and_elapsed_over_plain_thinking() {
        let line = text(
            &header_cells(60, "c", true, 0, true, Some(("coder", 12, (9, 9, 9))), 0),
            0,
        );
        assert!(
            line.contains("coder \u{00b7} 12s"),
            "active missing: {line}"
        );
        assert!(!line.contains("thinking"), "plain spinner leaked: {line}");
    }

    #[test]
    fn token_meter_appears_once_spend_is_nonzero() {
        assert!(!text(&header_cells(60, "c", true, 0, false, None, 0), 0).contains("tok"));
        let line = text(&header_cells(60, "c", true, 0, false, None, 9_500), 0);
        assert!(line.contains("~9.5k tok"), "meter missing: {line}");
    }

    #[test]
    fn all_cells_stay_within_width() {
        let cells = header_cells(
            20,
            "a-very-long-channel-name",
            true,
            999,
            true,
            Some(("x", 5, (9, 9, 9))),
            12345,
        );
        assert!(cells.iter().all(|c| c.col < 20 && c.row == 0));
    }
}
