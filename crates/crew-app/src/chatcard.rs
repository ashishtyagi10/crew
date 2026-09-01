//! One message card's identity: its gutter glyph, the colour its sender reads
//! in, the header line, and the two predicates that decide whether a card is
//! the system voice or a tool talking on an agent's behalf.
//!
//! Split from [`crate::chatmsgs`] for the line cap, along the line between
//! WHOSE card this is and how its body is laid out.
#[cfg(test)]
#[path = "chatcard_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatfade_tests.rs"]
mod fade_tests;

use crate::chatbody::{plain, CardCell, CardLine, Color};
use crate::chatlayout::Message;

/// The card header's gutter glyph (▍), in the sender's colour.
pub(crate) const GUTTER: char = '\u{258d}';

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

/// Whether `sender` is the broker/system voice — the telemetry senders that
/// share the dotted gutter, the muted ink and (see `chatfold`) the auto-fold.
/// ONE predicate so the four surfaces can never drift on the sender set.
pub(crate) fn is_system_voice(sender: &str) -> bool {
    matches!(sender, "agent smith" | "crew" | "system" | "broker")
}

/// The prefix both engines stamp on a tool line — `broker::toolcall` on the
/// relay, `broker::swarmmsg` in the swarm.
pub(crate) const TOOL_PREFIX: &str = "[tool] ";

/// Whether this card is a tool call or its result rather than something an
/// agent SAID.
///
/// It arrives under the agent's own name — which is right, you need to know
/// who reached for the tool — and that used to mean it rendered as a full
/// reply: solid gutter, the agent's roster colour, never folded. A task that
/// makes four calls then produced nine cards that all looked like the agent
/// talking, and the one card that was the answer had nothing to distinguish
/// it. This is the predicate that separates the two; the sender stays.
pub(crate) fn is_tool_card(m: &Message) -> bool {
    !is_system_voice(&m.sender) && m.text.starts_with(TOOL_PREFIX)
}

/// The gutter glyph for a card: a lighter bar for the system/broker voice and
/// for machine chatter (tool calls), the solid bar for what an agent or the
/// user actually said.
pub(crate) fn gutter_for(m: &Message) -> char {
    if is_system_voice(&m.sender) || is_tool_card(m) {
        '\u{2506}' // ┆ dotted — quieter
    } else {
        GUTTER // ▍ solid
    }
}

/// The colour a sender renders in: the broker/system voice is muted; every
/// agent (and the user) gets its stable roster colour.
pub(crate) fn sender_color(sender: &str) -> Color {
    if is_system_voice(sender) {
        crew_theme::theme().text_muted
    } else {
        crate::chatroster::agent_color(sender)
    }
}

/// …and the colour for one CARD, which is the sender's voice unless the card
/// is the machine talking on their behalf. `text_muted` is reused rather than
/// a new role invented: it is already contrast-checked against every page and
/// wash in the theme system, and a fourth ink for tools would have to earn
/// that all over again.
pub(crate) fn card_color(m: &Message, sender: &str) -> Color {
    if is_tool_card(m) {
        crew_theme::theme().text_muted
    } else {
        sender_color(sender)
    }
}

/// The `▍sender · 2m ago` header line. Multi-part senders (`a → b`) colour
/// each name separately with a muted arrow, so hand-offs read as from → to;
/// the muted tail carries only the relative time (the per-card reply latency
/// was dropped in the reductionist pass — one signal per question).
/// `connector` marks a follow-up card of the same task as the card above: it
/// swaps the gutter for a muted tree connector — `├` while more replies of
/// this task follow, `└` on the last one — and drops the repeated `#id` tag
/// (the chain root already carries it), so one task's replies read as one
/// thread (Claude-Code background-agent tree look).
pub(crate) fn header_line(m: &Message, now_ms: u64, connector: Option<char>) -> CardLine {
    let muted = crew_theme::theme().text_muted;
    let mut line: CardLine = Vec::new();
    let parts: Vec<&str> = m.sender.split(" \u{2192} ").collect();
    if let Some(conn) = connector {
        line.extend(format!("{conn} ").chars().map(|c| plain(c, muted, false)));
    } else {
        line.push(plain(gutter_for(m), card_color(m, parts[0]), false));
        if let Some(id) = crate::chattime::task_tag(&m.meta) {
            line.extend(format!("#{id} ").chars().map(|c| plain(c, muted, false)));
        }
    }
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            line.extend(" \u{2192} ".chars().map(|c| plain(c, muted, false)));
        }
        line.extend(part.chars().map(|c| plain(c, card_color(m, part), true)));
    }
    if let Some(rel) = crate::chattime::rel_time(&m.ts, now_ms) {
        let tail = format!(" \u{00b7} {rel}");
        line.extend(tail.chars().map(|c| plain(c, muted, false)));
    }
    line
}

/// The broker's Agent-Smith startup splash — the boxed nameplate art, spotted
/// by its `╔` top-left corner in the system voice. It renders header-less and
/// centered (see `splash_style`).
pub(crate) fn is_splash(m: &Message) -> bool {
    is_system_voice(&m.sender) && m.text.starts_with('\u{2554}')
}

/// Style the splash body in place: center every line in `cols`. Width-only,
/// so line counts can never drift between the counting and drawing passes.
pub(crate) fn splash_style(body: &mut [CardLine], cols: usize) {
    let muted = crew_theme::theme().text_muted;
    for line in body.iter_mut() {
        if line.is_empty() {
            continue;
        }
        let w: usize = line.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
        let pad = cols.saturating_sub(w) / 2;
        if pad > 0 {
            let fill: Vec<CardCell> = (0..pad).map(|_| plain(' ', muted, false)).collect();
            line.splice(0..0, fill);
        }
    }
}
