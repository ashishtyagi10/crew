//! Role-styled message cards for the crew pane: each message renders as a
//! `▍sender · 2m ago · 4.2s` header line in the sender's stable colour, with
//! the body beneath it (newline-aware prose, bordered code blocks — see
//! `chatbody`) and a blank spacer line between messages. Hand-off senders
//! (`planner → coder`) keep a per-name colour on each side.
use crew_render::CellView;

#[cfg(test)]
use crate::chatbody::CardCell;
use crate::chatbody::{body_lines, plain, CardLine, Color};
use crate::chatlayout::Message;
use crate::chatplace::{line_cells, window};

// Re-exported so this module's own tests reach it as `placed_lines` via
// `use super::*`, even though the placement logic itself lives in
// `chatplace` alongside the windowing helpers `message_cells` shares with it.
// `chatview::link_at` imports it from `chatplace` directly.
#[allow(unused_imports)]
pub(crate) use crate::chatplace::placed_lines;

/// The card header's gutter glyph (▍), in the sender's colour.
const GUTTER: char = '\u{258d}';

/// How long a freshly-arrived card takes to fade in from the page colour.
pub(crate) const FADE_MS: u64 = 400;

/// Fade progress for a message stamped `ts` (epoch ms): 0.0 just landed,
/// 1.0 fully drawn. Unparseable stamps and the counting pass (`now_ms == 0`)
/// render fully drawn.
pub(crate) fn fade_t(ts: &str, now_ms: u64) -> f32 {
    if now_ms == 0 {
        return 1.0;
    }
    let Ok(ts) = ts.parse::<u64>() else {
        return 1.0;
    };
    let age = now_ms.saturating_sub(ts);
    (age as f32 / FADE_MS as f32).min(1.0)
}

/// The card-line render mode: `source` shows raw text instead of markdown
/// (Ctrl+Shift+M, `ChatPane::show_source`); `compact` clamps each message to
/// its header line plus first body line only (Ctrl+O,
/// `ChatPane::compact_view`). Threaded as one value through
/// `card_lines`/`card_line_count`/`message_cells`/`chatplace::placed_lines`
/// so scroll math, the scrollbar, link hit-tests and the unread pill all
/// agree on the same rendering automatically. The two flags are orthogonal —
/// both can be on at once (raw text, one line) — so this is a plain copy
/// struct, not an enum.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct View {
    pub(crate) source: bool,
    pub(crate) compact: bool,
}

/// The gutter glyph for a sender: a lighter bar for the system/broker voice,
/// the solid bar for agents and the user.
fn gutter_for(sender: &str) -> char {
    match sender {
        "crew" | "system" | "broker" => '\u{2506}', // ┆ dotted — quieter
        _ => GUTTER,                                // ▍ solid
    }
}

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
    line.push(plain(gutter_for(&m.sender), sender_color(parts[0]), false));
    if let Some(id) = crate::chattime::task_tag(&m.meta) {
        for c in format!("#{id} ").chars() {
            line.push(plain(c, muted, false));
        }
    }
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            line.extend(" \u{2192} ".chars().map(|c| plain(c, muted, false)));
        }
        line.extend(part.chars().map(|c| plain(c, sender_color(part), true)));
    }
    let tail = crate::chattime::meta_suffix(&m.ts, &m.meta, now_ms);
    line.extend(tail.chars().map(|c| plain(c, muted, false)));
    line
}

/// Appends a muted ` … +N` suffix (`hidden` = number of clamped-away body
/// lines) to a compact-clamped first body line, trimming trailing cells so
/// the line — plus the suffix — still fits `cols` display columns. Mirrors
/// the width-clamp rule other suffixes in this module apply at render time
/// (`line_cells` would otherwise silently drop overflow, risking a
/// partially-cut suffix rather than a clean truncation of the body text).
fn append_hidden_suffix(line: &mut CardLine, hidden: usize, cols: usize) {
    let muted = crew_theme::theme().text_muted;
    let suffix = format!(" \u{2026} +{hidden}");
    let suffix_w: usize = suffix.chars().map(crate::chatwidth::char_w).sum();
    let mut w: usize = line.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
    while w + suffix_w > cols {
        match line.pop() {
            Some(cell) => w -= crate::chatwidth::char_w(cell.c),
            None => break,
        }
    }
    line.extend(suffix.chars().map(|c| plain(c, muted, false)));
}

/// All messages as card lines: header, body, spacer between cards. Visible
/// to `chatplace` so `placed_lines` can build the same lines `message_cells`
/// draws. `view.source` shows plain text instead of markdown; `view.compact`
/// clamps each message's body to its first line, appending a muted ` … +N`
/// suffix when lines were hidden (single-line bodies render unchanged).
pub(crate) fn card_lines(
    messages: &[Message],
    cols: usize,
    now_ms: u64,
    view: View,
) -> Vec<CardLine> {
    let mut out: Vec<CardLine> = Vec::new();
    for (i, m) in messages.iter().enumerate() {
        if i > 0 {
            out.push(Vec::new()); // spacer between cards
        }
        let first = out.len();
        out.push(header_line(m, now_ms));
        // Body text: agents speak in ink; the system voice stays muted.
        let fg = match m.sender.as_str() {
            "crew" | "system" | "broker" => crew_theme::theme().text_muted,
            _ => crew_theme::theme().ink,
        };
        let mut body = body_lines(&m.text, cols, fg, view.source);
        if view.compact && body.len() > 1 {
            let hidden = body.len() - 1;
            body.truncate(1);
            append_hidden_suffix(&mut body[0], hidden, cols);
        }
        out.extend(body);
        // A just-landed card fades in from the page colour (see `fade_t`).
        let t = fade_t(&m.ts, now_ms);
        if t < 1.0 {
            let page = crew_theme::theme().page_bg;
            for line in &mut out[first..] {
                for cell in line.iter_mut() {
                    cell.fg = crate::anim::lerp_rgb(page, cell.fg, t);
                }
            }
        }
    }
    out
}

/// Total card lines for the given width — the scroll clamp for the card view.
pub(crate) fn card_line_count(messages: &[Message], cols: u16, view: View) -> usize {
    if cols == 0 {
        return 0;
    }
    card_lines(messages, cols as usize, 0, view).len()
}

/// Render the card view of `messages` into `rows` rows starting at `top_row`,
/// scrolled `scroll` lines up from the live bottom, in the given render `view`
/// (see [`View`]).
pub(crate) fn message_cells(
    messages: &[Message],
    cols: u16,
    rows: u16,
    top_row: u16,
    scroll: usize,
    view: View,
) -> Vec<CellView> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    let page = crew_theme::theme().page_bg;
    let lines = card_lines(
        messages,
        cols as usize,
        crate::chattime::unix_now_ms(),
        view,
    );
    window(lines, rows, top_row, scroll)
        .iter()
        .flat_map(|(row, line)| line_cells(*row, line, cols, page))
        .collect()
}

#[cfg(test)]
#[path = "chatmsgs_tests.rs"]
mod tests;
