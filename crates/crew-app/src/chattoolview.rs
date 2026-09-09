//! Where an agent's tool block draws in the transcript (see
//! [`crate::chattool`]): directly under the agent's streaming card, ABOVE
//! its settled reply once that lands (collapsed to one summary line, click
//! to open), or — with no card of the agent's on screen — as its own thin
//! card headed by the agent's badge, after everything else. The placement
//! predicates here are the ones `chatmsgs::card_lines_spanned` draws with
//! AND the ones `chattoolfold` resolves a click against, so a row can never
//! be attributed to a block the frame did not put there.
use crate::chatbody::{plain, CardLine};
use crate::chatlayout::Message;
use crate::chatmsgs::View;
use crate::chattool::ToolBlock;
use crate::chattoolline::{glyph, render, text_rows};
use crate::glyphs::Glyph;

/// What a click on one of a block's rows lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ToolHit {
    /// The collapsed/expanded summary line.
    Summary,
    /// Call `i`'s line, or one of its opened text rows.
    Line(usize),
}

/// `  ▸ 4 tool calls · 2.1 s` (`▾` open; a wrench on the icon set) with a
/// `· 1 failed` in the removed ink when any call did.
pub(crate) fn summary(b: &ToolBlock, cols: usize, on: bool) -> CardLine {
    let th = crew_theme::theme();
    let (calls, failed, ms) = b.tally();
    let mark = glyph(
        if b.expanded {
            Glyph::ToolOpen
        } else {
            Glyph::Tool
        },
        on,
    );
    let plural = if calls == 1 { "" } else { "s" };
    let head = format!(
        "  {mark} {calls} tool call{plural} \u{00b7} {}",
        crate::chattoolline::fmt_ms(ms)
    );
    let mut out: CardLine = head
        .chars()
        .map(|c| plain(c, th.text_muted, false))
        .collect();
    if failed > 0 {
        let fg = crate::chatink::token_fg(crate::md::syntax::Token::Removed);
        let tail = format!(" \u{00b7} {failed} failed");
        out.extend(tail.chars().map(|c| plain(c, fg, false)));
    }
    let w: usize = out.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
    if w > cols {
        out.truncate(cols);
    }
    out
}

/// A thin card's header: the dotted gutter and the agent's badge.
pub(crate) fn header(b: &ToolBlock) -> CardLine {
    let color = crate::chatroster::agent_color(&b.agent);
    let mut line = vec![plain('\u{2506}', color, false)];
    let badge = crate::segment::badge(
        &b.agent,
        crate::segment::page_ink(color),
        color,
        crate::segment::Caps::BOTH,
    );
    line.extend(crate::segment::to_card(&badge));
    line
}

/// The block's rows: every line (plus opened text) while live; the summary
/// alone once settled, the summary then the lines when clicked open.
pub(crate) fn block_lines(b: &ToolBlock, now_ms: u64, cols: usize, on: bool) -> Vec<CardLine> {
    let mut out = Vec::new();
    if b.settled {
        out.push(summary(b, cols, on));
        if !b.expanded {
            return out;
        }
    }
    for l in &b.lines {
        out.push(render(l, now_ms, cols, on));
        out.extend(text_rows(l, cols));
    }
    out
}

/// Which of the block's [`block_lines`] rows `offset` is — the same walk,
/// so the two cannot disagree.
pub(crate) fn hit(b: &ToolBlock, cols: usize, offset: usize) -> Option<ToolHit> {
    let mut at = 0;
    if b.settled {
        if offset == 0 {
            return Some(ToolHit::Summary);
        }
        if !b.expanded {
            return None;
        }
        at = 1;
    }
    for (i, l) in b.lines.iter().enumerate() {
        let n = 1 + text_rows(l, cols).len();
        if offset < at + n {
            return Some(ToolHit::Line(i));
        }
        at += n;
    }
    None
}

/// The block anchored above settled card `m`, if any.
pub(crate) fn above_of(view: View<'_>, m: &Message, streaming: bool) -> Option<usize> {
    if streaming {
        return None;
    }
    let key = crate::chatflow::stream_key(&m.sender);
    view.tools
        .iter()
        .position(|b| b.agent == key && b.anchor.as_deref() == Some(m.ts.as_str()))
}

/// The live block under streaming card `m`, if any.
pub(crate) fn below_of(view: View<'_>, m: &Message, streaming: bool) -> Option<usize> {
    if !streaming {
        return None;
    }
    let key = crate::chatflow::stream_key(&m.sender);
    view.tools.iter().position(|b| !b.settled && b.agent == key)
}

/// The blocks no card seats — never anchored, and no streaming card of
/// their agent on screen — in the order they stand at the end.
pub(crate) fn orphan_ids(view: View<'_>, messages: &[&Message]) -> Vec<usize> {
    (0..view.tools.len())
        .filter(|&i| {
            let placed = messages.iter().enumerate().any(|(mi, m)| {
                let s = mi >= view.streaming_from;
                above_of(view, m, s) == Some(i) || below_of(view, m, s) == Some(i)
            });
            !placed && view.tools[i].anchor.is_none()
        })
        .collect()
}

fn lines_of(view: View<'_>, i: usize, now_ms: u64, cols: usize) -> Vec<CardLine> {
    block_lines(&view.tools[i], now_ms, cols, crate::glyphs::on())
}

/// The rows to put above settled card `m`'s header.
pub(crate) fn above(
    view: View<'_>,
    m: &Message,
    streaming: bool,
    now: u64,
    cols: usize,
) -> Vec<CardLine> {
    above_of(view, m, streaming).map_or(Vec::new(), |i| lines_of(view, i, now, cols))
}

/// The rows to put under streaming card `m`'s body.
pub(crate) fn below(
    view: View<'_>,
    m: &Message,
    streaming: bool,
    now: u64,
    cols: usize,
) -> Vec<CardLine> {
    below_of(view, m, streaming).map_or(Vec::new(), |i| lines_of(view, i, now, cols))
}

/// The unseated blocks as thin cards after the transcript: a card gap (when
/// anything precedes), the agent's header, the block's rows.
pub(crate) fn orphans(
    view: View<'_>,
    messages: &[&Message],
    now: u64,
    cols: usize,
) -> Vec<CardLine> {
    let mut out = Vec::new();
    for i in orphan_ids(view, messages) {
        if !messages.is_empty() || !out.is_empty() {
            out.extend(std::iter::repeat_with(Vec::new).take(view.gap_rows));
        }
        out.push(header(&view.tools[i]));
        out.extend(lines_of(view, i, now, cols));
    }
    out
}

#[cfg(test)]
#[path = "chattoolview_tests.rs"]
mod tests;
