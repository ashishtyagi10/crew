//! How an agent's thought draws (see [`crate::chatthought`]): the live
//! block's header and tail while the working arrives, the folded row and the
//! opened text once it settled, and the thin card's badge header. Where the
//! rows go is `chatthoughtseat`.
//!
//! The working is drawn as plain wrapped text in the muted italic ink, never
//! through the markdown engine: it is scratch, and rendering a half-formed
//! list or fence in it would give it the weight of an answer.
use crate::chatbody::{plain, CardCell, CardLine};
use crate::chatthought::{Live, ThoughtBlock, LIVE_ROWS, THOUGHT_ROWS};

fn ital(c: char, fg: crate::chatbody::Color) -> CardCell {
    CardCell {
        italic: true,
        ..plain(c, fg, false)
    }
}

/// The working, wrapped plain to `width`, blank lines dropped.
fn wrap(text: &str, width: usize) -> Vec<String> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .flat_map(|l| crate::toolsrow::wrap(l.trim(), width.max(1)))
        .collect()
}

fn body_rows(lines: &[String], cols: usize) -> Vec<CardLine> {
    let muted = crew_theme::theme().text_muted;
    lines
        .iter()
        .map(|l| {
            let s = format!("  {}", crate::chatwidth::clip_w(l, cols.saturating_sub(2)));
            s.chars().map(|c| ital(c, muted)).collect()
        })
        .collect()
}

/// `  ∴ thinking · 3s` — the word shimmers like the header's (plain at
/// motion Off) — then the last [`LIVE_ROWS`] lines of the working. The clock
/// shows once a second has passed, never at the counting pass (`now_ms == 0`),
/// so both passes agree on the row count.
pub(crate) fn live_lines(l: &Live, now_ms: u64, cols: usize) -> Vec<CardLine> {
    let th = crew_theme::theme();
    let mut head: CardLine = "  "
        .chars()
        .map(|c| plain(c, th.text_muted, false))
        .collect();
    head.push(plain('\u{2234}', crate::palette::accent(), false));
    head.push(plain(' ', th.text_muted, false));
    let level = crate::motion::level();
    let ms = crate::shimmer::SHIMMER_MS;
    let word = crate::shimmer::cells(
        "thinking",
        now_ms,
        th.text_muted,
        crate::palette::accent(),
        th.page_bg,
        ms,
        level,
    );
    head.extend(word.into_iter().map(|(c, fg)| plain(c, fg, false)));
    let secs = now_ms.saturating_sub(l.since_ms) / 1000;
    if now_ms > 0 && secs > 0 {
        let tail = format!(" \u{00b7} {secs}s");
        head.extend(tail.chars().map(|c| plain(c, th.text_muted, false)));
    }
    let lines = wrap(&l.text, cols.saturating_sub(2));
    let from = lines.len().saturating_sub(LIVE_ROWS);
    let mut out = vec![head];
    out.extend(body_rows(&lines[from..], cols));
    out
}

/// `  ▸ thought for 4.2 s · 812 chars` (`▾` open); `thought · N chars` for
/// one that arrived whole.
pub(crate) fn summary(b: &ThoughtBlock, cols: usize) -> CardLine {
    let muted = crew_theme::theme().text_muted;
    let mark = if b.expanded { '\u{25be}' } else { '\u{25b8}' };
    let span = match b.ms {
        0 => String::new(),
        ms => format!(" for {}", crate::chattoolline::fmt_ms(ms)),
    };
    let s = format!(
        "  {mark} thought{span} \u{00b7} {} chars",
        b.text.trim().chars().count()
    );
    crate::chatwidth::clip_w(&s, cols)
        .chars()
        .map(|c| plain(c, muted, false))
        .collect()
}

/// The folded row; then, when clicked open, the working up to
/// [`THOUGHT_ROWS`] rows, the last one `… +N lines` when it was longer.
pub(crate) fn block_lines(b: &ThoughtBlock, cols: usize) -> Vec<CardLine> {
    let mut out = vec![summary(b, cols)];
    if !b.expanded {
        return out;
    }
    let mut lines = wrap(&b.text, cols.saturating_sub(2));
    if lines.len() > THOUGHT_ROWS {
        let rest = lines.len() - (THOUGHT_ROWS - 1);
        lines.truncate(THOUGHT_ROWS - 1);
        lines.push(format!("\u{2026} +{rest} lines"));
    }
    out.extend(body_rows(&lines, cols));
    out
}

/// A thin card's header: the dotted gutter and the agent's badge.
pub(crate) fn header(agent: &str) -> CardLine {
    let color = crate::chatroster::agent_color(agent);
    let mut line = vec![plain('\u{2506}', color, false)];
    let badge = crate::segment::badge(
        agent,
        crate::segment::page_ink(color),
        color,
        crate::segment::Caps::BOTH,
    );
    line.extend(crate::segment::to_card(&badge));
    line
}

#[cfg(test)]
#[path = "chatthoughtview_tests.rs"]
mod tests;
