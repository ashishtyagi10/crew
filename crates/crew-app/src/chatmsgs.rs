//! Role-styled message cards for the crew pane: each message renders as a
//! `▍sender` header line in the sender's stable colour, with the wrapped body
//! indented beneath it and a blank spacer line between messages — so who said
//! what is legible at a glance, unlike the old inline `sender: text` stream.
//! Hand-off senders (`planner → coder`) keep a per-name colour on each side.
use crew_render::CellView;

use crate::chatlayout::{wrap_indices, Message};

type Color = (u8, u8, u8);
/// One rendered line: `(char, colour, bold)` cells.
type CardLine = Vec<(char, Color, bool)>;

/// The card header's gutter glyph (▍), in the sender's colour.
const GUTTER: char = '\u{258d}';

/// The colour a sender renders in: the broker/system voice is muted; every
/// agent (and the user) gets its stable roster colour.
fn sender_color(sender: &str) -> Color {
    match sender {
        "crew" | "system" | "broker" => crew_theme::theme().text_muted,
        _ => crate::chatroster::agent_color(sender),
    }
}

/// The `▍sender · 2m ago · 4.2s` header line. Multi-part senders (`a → b`)
/// colour each name separately with a muted arrow, so hand-offs read as
/// from → to; the muted tail carries the relative time and reply latency.
fn header_line(m: &Message, now_ms: u64) -> CardLine {
    let muted = crew_theme::theme().text_muted;
    let mut line: CardLine = Vec::new();
    let parts: Vec<&str> = m.sender.split(" \u{2192} ").collect();
    line.push((GUTTER, sender_color(parts[0]), false));
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            for c in " \u{2192} ".chars() {
                line.push((c, muted, false));
            }
        }
        for c in part.chars() {
            line.push((c, sender_color(part), true));
        }
    }
    for c in crate::chattime::meta_suffix(&m.ts, &m.meta, now_ms).chars() {
        line.push((c, muted, false));
    }
    line
}

/// All messages as card lines: header, indented body, spacer between cards.
fn card_lines(messages: &[Message], cols: usize, now_ms: u64) -> Vec<CardLine> {
    let mut out: Vec<CardLine> = Vec::new();
    for (i, m) in messages.iter().enumerate() {
        if i > 0 {
            out.push(Vec::new()); // spacer between cards
        }
        out.push(header_line(m, now_ms));
        // Body text: agents speak in ink; the system voice stays muted.
        let fg = match m.sender.as_str() {
            "crew" | "system" | "broker" => crew_theme::theme().text_muted,
            _ => crew_theme::theme().ink,
        };
        let body_cols = cols.saturating_sub(1).max(1);
        let full: Vec<char> = m.text.chars().collect();
        for (s, e) in wrap_indices(&full, body_cols) {
            let mut line: CardLine = vec![(' ', fg, false)];
            line.extend(full[s..e].iter().map(|&c| (c, fg, false)));
            out.push(line);
        }
    }
    out
}

/// Total card lines for the given width — the scroll clamp for the card view.
pub(crate) fn card_line_count(messages: &[Message], cols: u16) -> usize {
    if cols == 0 {
        return 0;
    }
    card_lines(messages, cols as usize, 0).len()
}

/// Render the card view of `messages` into `rows` rows starting at `top_row`,
/// scrolled `scroll` lines up from the live bottom.
pub(crate) fn message_cells(
    messages: &[Message],
    cols: u16,
    rows: u16,
    top_row: u16,
    scroll: usize,
) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let bg = crew_theme::theme().page_bg;
    let lines = card_lines(messages, cols as usize, crate::chattime::unix_now_ms());
    let max_start = lines.len().saturating_sub(rows as usize);
    let start = max_start.saturating_sub(scroll);
    let end = (start + rows as usize).min(lines.len());
    let mut cells = Vec::new();
    for (row_offset, line) in lines[start..end].iter().enumerate() {
        for (col, &(c, fg, bold)) in line.iter().enumerate() {
            if col as u16 >= cols {
                break;
            }
            cells.push(CellView {
                col: col as u16,
                row: top_row + row_offset as u16,
                c,
                fg,
                bg,
                bold,
                italic: false,
            });
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_tail_carries_latency_metadata() {
        let mut m = msg("coder", "done");
        m.meta = "4.2s".into();
        let line = header_line(&m, 0);
        let text: String = line.iter().map(|&(c, ..)| c).collect();
        assert!(text.ends_with("\u{00b7} 4.2s"), "got: {text}");
    }

    fn msg(sender: &str, text: &str) -> Message {
        Message {
            sender: sender.into(),
            text: text.into(),
            ts: String::new(),
            meta: String::new(),
        }
    }

    fn row_text(cells: &[CellView], row: u16) -> String {
        let mut v: Vec<(u16, char)> = cells
            .iter()
            .filter(|c| c.row == row)
            .map(|c| (c.col, c.c))
            .collect();
        v.sort_unstable();
        v.into_iter().map(|(_, c)| c).collect()
    }

    #[test]
    fn card_has_header_then_indented_body() {
        let cells = message_cells(&[msg("planner", "hello")], 40, 10, 0, 0);
        assert_eq!(row_text(&cells, 0), format!("{GUTTER}planner"));
        assert_eq!(row_text(&cells, 1), " hello");
    }

    #[test]
    fn cards_are_separated_by_a_blank_line() {
        let m = [msg("planner", "a"), msg("coder", "b")];
        let cells = message_cells(&m, 40, 10, 0, 0);
        assert_eq!(row_text(&cells, 2), ""); // spacer
        assert_eq!(row_text(&cells, 3), format!("{GUTTER}coder"));
    }

    #[test]
    fn handoff_sender_colours_each_name_separately() {
        let line = header_line(&msg("planner \u{2192} coder", ""), 0);
        let text: String = line.iter().map(|&(c, ..)| c).collect();
        assert_eq!(text, format!("{GUTTER}planner \u{2192} coder"));
        let planner_fg = line[1].1;
        let coder_fg = line.last().unwrap().1;
        assert_ne!(planner_fg, crew_theme::theme().text_muted);
        assert_ne!(coder_fg, crew_theme::theme().text_muted);
    }

    #[test]
    fn system_sender_is_muted_and_agents_are_not() {
        assert_eq!(sender_color("crew"), crew_theme::theme().text_muted);
        assert_ne!(sender_color("planner"), crew_theme::theme().text_muted);
    }

    #[test]
    fn count_matches_rendered_lines_and_scroll_shows_older() {
        let m = [msg("a", "one"), msg("b", "two")];
        // 2 cards × (header + body) + 1 spacer = 5 lines.
        assert_eq!(card_line_count(&m, 40), 5);
        // A 2-row window scrolled 3 up from the bottom shows the first card.
        let cells = message_cells(&m, 40, 2, 0, 3);
        assert_eq!(row_text(&cells, 0), format!("{GUTTER}a"));
    }

    #[test]
    fn top_row_offsets_and_width_clips() {
        let cells = message_cells(&[msg("planner", "wide text here")], 5, 4, 3, 0);
        assert!(cells.iter().all(|c| c.row >= 3 && c.col < 5));
    }
}
