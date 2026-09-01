//! Role-styled message cards for the crew pane: each message renders as a
//! `▍sender · 2m ago` header line in the sender's stable colour, with
//! the body beneath it (newline-aware prose, bordered code blocks — see
//! `chatbody`) and a blank spacer line between messages. Hand-off senders
//! (`planner → coder`) keep a per-name colour on each side.
pub(crate) use crate::chatcard::*;
use crew_render::CellView;

use crate::chatbody::{body_lines, plain, CardLine};
use crate::chatlayout::Message;
use crate::chatplace::{line_cells, window};

// Re-exported so this module's own tests reach it as `placed_lines` via
// `use super::*`, even though the placement logic itself lives in
// `chatplace` alongside the windowing helpers `message_cells` shares with it.
// `chatview::link_at` imports it from `chatplace` directly.
#[allow(unused_imports)]
pub(crate) use crate::chatplace::placed_lines;

/// The card-line render mode: `source` shows raw text instead of markdown
/// (Ctrl+Shift+M, `ChatPane::show_source`); `compact` clamps each message to
/// its header line plus first body line only (Ctrl+O,
/// `ChatPane::compact_view`). Threaded as one value through
/// `card_lines`/`card_line_count`/`message_cells`/`chatplace::placed_lines`
/// so scroll math, the scrollbar, link hit-tests and the unread pill all
/// agree on the same rendering automatically. The two flags are orthogonal —
/// both can be on at once (raw text, one line) — so this is a plain copy
/// struct, not an enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct View {
    pub(crate) source: bool,
    pub(crate) compact: bool,
    /// Index at which still-streaming cards begin, in the same slice
    /// `visible_messages` returns. Those get a live caret on their last line.
    /// `usize::MAX` (the default) means nothing is in flight.
    pub(crate) streaming_from: usize,
    /// Blank rows to put between two unrelated cards — the Density setting's
    /// call (see [`crate::density::Density::card_gap_rows`]).
    ///
    /// Carried on the view rather than read from the density global at the
    /// point of use, unlike the pane gutter. The gutter has to be a global
    /// because render and hit-testing reach it down two different call
    /// stacks; this one has a struct already threaded through every layout
    /// path, and a per-call value is a value a test can set without reaching
    /// for a process-wide mutex.
    pub(crate) gap_rows: usize,
}

impl Default for View {
    fn default() -> Self {
        Self {
            source: false,
            compact: false,
            streaming_from: usize::MAX,
            gap_rows: crate::density::Density::Cozy.card_gap_rows(),
        }
    }
}

/// Period of the streaming caret's pulse. Slow enough to read as a live cursor
/// rather than a warning light.
const CARET_MS: u64 = 900;

/// Put a pulsing block on the end of a streaming card's last line — the one
/// unambiguous sign that text is still arriving, as distinct from a reply that
/// simply ended mid-sentence.
///
/// It pulses between the muted and accent colours rather than blinking on and
/// off: a caret that vanishes half the time reads, at a glance, like the text
/// stopped.
fn push_caret(lines: &mut [CardLine], now_ms: u64, cols: usize) {
    let Some(last) = lines.last_mut() else { return };
    if last.len() >= cols {
        return;
    }
    let t = match crate::motion::level() {
        crate::motion::MotionLevel::Off => 1.0,
        _ => crate::anim::tri(now_ms, CARET_MS),
    };
    let th = crew_theme::theme();
    let fg = crate::anim::lerp_rgb(th.text_muted, crate::palette::accent(), t);
    last.push(crate::chatbody::CardCell {
        c: '\u{258c}',
        fg,
        bold: false,
        italic: false,
        bg: None,
        link: None,
        src: None,
    });
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

/// The full (unclamped) body of one message — markdown or raw per
/// `view.source`, plus the usage trailer. The single input both the compact
/// clamp and the auto-fold measure (`chatfold` reads its length to decide
/// whether a card is long enough to fold), so the two can never disagree
/// with what `card_lines` actually renders.
pub(crate) fn full_body(m: &Message, cols: usize, view: View) -> Vec<CardLine> {
    // Body text: agents speak in ink; the system voice — and the machine
    // talking on an agent's behalf — stays muted.
    let fg = if is_system_voice(&m.sender) || is_tool_card(m) {
        crew_theme::theme().text_muted
    } else {
        crew_theme::theme().ink
    };
    // The `[tool] ` marker is MACHINERY: it is how the broker tells the app
    // what kind of card this is, and it is carried in the text because that
    // is the only field that crosses the wire. Now that the gutter and the
    // ink say the same thing in a glyph, printing it too is telling the
    // reader twice. Stripped here, in the ONE place both the counting and the
    // drawing pass read — strip it anywhere else and the two disagree about
    // where the card wraps.
    let text = match is_tool_card(m) {
        true => m.text.strip_prefix(TOOL_PREFIX).unwrap_or(&m.text),
        false => m.text.as_str(),
    };
    let mut body = body_lines(text, cols, fg, view.source);
    // The reply's usage trailer joins the body BEFORE the clamp in normal
    // view, so the auto-fold hides it — and counts it in ` … +N` — like any
    // body line. Compact view (Ctrl+O) excludes it entirely: it is metadata,
    // not content, and counting it would stamp a misleading ` … +1` on every
    // single-line reply that carries usage.
    if !view.compact {
        if let Some(t) = m.usage.and_then(crate::chatusage::trailer_line) {
            body.push(t);
        }
    }
    body
}

/// All messages as card lines: header, body, spacer between cards. Visible
/// to `chatplace` so `placed_lines` can build the same lines `message_cells`
/// draws. `view.source` shows plain text instead of markdown; `view.compact`
/// clamps each message's body to its first line, appending a muted ` … +N`
/// suffix when lines were hidden (single-line bodies render unchanged); in
/// normal view long system-voice cards get the same clamp until clicked open
/// (`chatfold::folded`).
pub(crate) fn card_lines(
    messages: &[&Message],
    cols: usize,
    now_ms: u64,
    view: View,
) -> Vec<CardLine> {
    card_lines_spanned(messages, cols, now_ms, view).0
}

/// [`card_lines`] plus each message's `[start, end)` line span in the result
/// (spacers belong to no message). The spans are recorded while the lines are
/// built — never re-derived — so `chatfold`'s click hit-test attributes rows
/// to messages with the exact layout the frame drew.
pub(crate) fn card_lines_spanned(
    messages: &[&Message],
    cols: usize,
    now_ms: u64,
    view: View,
) -> (Vec<CardLine>, Vec<std::ops::Range<usize>>) {
    let mut out: Vec<CardLine> = Vec::new();
    let mut spans: Vec<std::ops::Range<usize>> = Vec::with_capacity(messages.len());
    for (i, m) in messages.iter().enumerate() {
        // A card continuing the task of the card above chains onto it: no
        // spacer, and its header renders a tree connector instead of the
        // gutter (see `header_line`), so one task's replies read as a thread.
        let tid = crate::chattime::task_tag(&m.meta);
        let chained =
            tid.is_some() && i > 0 && tid == crate::chattime::task_tag(&messages[i - 1].meta);
        let continues = tid.is_some()
            && messages
                .get(i + 1)
                .is_some_and(|n| tid == crate::chattime::task_tag(&n.meta));
        // ├ while more replies of this task follow, └ on the last — the
        // Claude-Code tree look, so a task's replies read as one thread.
        let connector = chained.then_some(if continues { '\u{251c}' } else { '\u{2514}' });
        if i > 0 && !chained {
            // Spacer between unrelated cards — how many rows is the Density
            // setting's call (`compact` takes none: the header's coloured
            // gutter glyph already draws the boundary in ink). Chained
            // replies never take one at any density; the tree connector is
            // what says they belong together.
            for _ in 0..view.gap_rows {
                out.push(Vec::new());
            }
        }
        let first = out.len();
        let splash = is_splash(m);
        if !splash {
            out.push(header_line(m, now_ms, connector));
        }
        let mut body = full_body(m, cols, view);
        // One clamp, two triggers: pane-global compact view (Ctrl+O, which
        // wins outright), or — in normal view — a long system-voice card the
        // user hasn't clicked open (`chatfold::folded`).
        let clamp = if view.compact {
            body.len() > 1
        } else {
            crate::chatfold::folded(m, body.len())
        };
        if clamp {
            let hidden = body.len() - 1;
            body.truncate(1);
            append_hidden_suffix(&mut body[0], hidden, cols);
        }
        if splash {
            splash_style(&mut body, cols);
        }
        if i >= view.streaming_from {
            push_caret(&mut body, now_ms, cols);
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
        spans.push(first..out.len());
    }
    (out, spans)
}

/// Total card lines for the given width — the scroll clamp for the card view.
pub(crate) fn card_line_count(messages: &[&Message], cols: u16, view: View) -> usize {
    if cols == 0 {
        return 0;
    }
    card_lines(messages, cols as usize, 0, view).len()
}

/// Render the card view of `messages` into `rows` rows starting at `top_row`,
/// scrolled `scroll` lines up from the live bottom, in the given render `view`
/// (see [`View`]).
pub(crate) fn message_cells(
    messages: &[&Message],
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
pub(crate) mod tests;
