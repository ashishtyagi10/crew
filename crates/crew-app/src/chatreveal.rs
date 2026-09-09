//! The typewriter: a streamed reply is REVEALED at a rate instead of landing
//! in the 80 ms word-group bursts the broker's text gate coalesces it into
//! (`crew_plugin::broker::tick::TEXT_GAP_MS`).
//!
//! Pure and clock-driven: a card's [`Reveal`] is a checkpoint (`shown` chars
//! at `at_ms`) and [`visible_len`] derives what is on screen NOW from it and a
//! rate — no timer, no thread, no per-frame state, so 15 fps
//! (`poll::BUSY_ANIM_DIV`) and 60 fps draw the same text at the same instant.
//!
//! The rate adapts to arrival: at least [`BASE_CPS`], fast enough to finish
//! whatever is pending within [`CATCHUP_MS`] of the last delta, and never
//! more than [`LAG_CHUNKS`] gate chunks behind what has actually arrived.
//! Motion `Off` reveals everything the instant it lands — a true no-op path —
//! and `Subtle` types at twice the pace.
use crate::chatbody::CardLine;
use crate::chatlayout::Message;
use crate::motion::MotionLevel;

/// The slowest the typewriter runs, in characters per second.
pub(crate) const BASE_CPS: f32 = 90.0;
/// Whatever is pending is on screen this long after the last delta.
pub(crate) const CATCHUP_MS: u64 = 400;
/// The visible text never trails arrival by more than this many gate chunks.
pub(crate) const LAG_CHUNKS: f32 = 1.5;
/// The broker's coalescing gate — the period a chunk stands for.
const GATE_MS: u64 = 80;
/// How many of the newest revealed characters carry the brightness ramp.
pub(crate) const GLOW_CHARS: usize = 12;
/// How long a just-revealed character takes to ramp from muted to its ink.
pub(crate) const GLOW_MS: u64 = 180;
/// Period of the streaming caret's pulse. Slow enough to read as a live cursor
/// rather than a warning light.
pub(crate) const CARET_MS: u64 = 900;

/// One card's reveal checkpoint. `shown` characters were on screen at
/// `at_ms`; `chunk` is the size of the delta that set the checkpoint (the
/// lag bound's unit); `settled` marks a card whose real `Message` has landed
/// and is finishing the reveal of the streamed text — no fade-in re-fires.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Reveal {
    pub(crate) shown: usize,
    pub(crate) at_ms: u64,
    pub(crate) chunk: usize,
    pub(crate) settled: bool,
}

impl Reveal {
    /// Re-checkpoint at `now` for a card whose text grew `old` → `new` chars.
    pub(crate) fn arrived(self, now: u64, old: usize, new: usize, level: MotionLevel) -> Self {
        Reveal {
            shown: visible_len(&self, now, old, level).min(new),
            at_ms: now,
            chunk: new.saturating_sub(old),
            settled: self.settled,
        }
    }

    /// The settled text (`new` chars) replaced the streamed one (`old`):
    /// continue from what was visible, finishing within [`CATCHUP_MS`]; a
    /// shorter settled text than what was visible shows at once.
    pub(crate) fn settle(self, now: u64, old: usize, new: usize, level: MotionLevel) -> Self {
        let shown = visible_len(&self, now, old, level).min(new);
        Reveal {
            shown,
            at_ms: now,
            chunk: new - shown,
            settled: true,
        }
    }
}

/// Characters per second the card types at right now.
pub(crate) fn rate_cps(state: &Reveal, total: usize, level: MotionLevel) -> f32 {
    let pending = total.saturating_sub(state.shown) as f32;
    let excess = (pending - LAG_CHUNKS * state.chunk as f32).max(0.0);
    let r = BASE_CPS
        .max(pending * 1000.0 / CATCHUP_MS as f32)
        .max(excess * 1000.0 / GATE_MS as f32);
    match level {
        MotionLevel::Subtle => r * 2.0,
        _ => r,
    }
}

/// How many of a `total`-character text are on screen at `now_ms`.
/// Monotonic in `now_ms`, never above `total`, all of it at motion `Off`.
pub(crate) fn visible_len(state: &Reveal, now_ms: u64, total: usize, level: MotionLevel) -> usize {
    if level == MotionLevel::Off || state.shown >= total {
        return total;
    }
    let dt_ms = now_ms.saturating_sub(state.at_ms) as f32;
    let typed = (rate_cps(state, total, level) * dt_ms / 1000.0).round() as usize;
    (state.shown + typed).min(total)
}

/// How long ago character `i` was revealed, under the checkpoint's rate.
fn char_age_ms(s: &Reveal, now: u64, i: usize, total: usize, level: MotionLevel) -> u64 {
    let since = now.saturating_sub(s.at_ms);
    let typed_ms = i.saturating_sub(s.shown) as f32 * 1000.0 / rate_cps(s, total, level);
    since.saturating_sub(typed_ms as u64)
}

/// The brightness mix for a character revealed `age_ms` ago: 0 = the muted
/// tone, 1 = its real ink. `Off` is always 1 — no per-character fade at all.
pub(crate) fn char_mix(age_ms: u64, level: MotionLevel) -> f32 {
    match level.scale_ms(GLOW_MS) {
        0 => 1.0,
        ms => (age_ms as f32 / ms as f32).min(1.0),
    }
}

/// The first `chars` characters of `text`, on a char boundary.
pub(crate) fn clip(text: &str, chars: usize) -> &str {
    match text.char_indices().nth(chars) {
        Some((b, _)) => &text[..b],
        None => text,
    }
}

/// `text` as revealed at `now` — raw, before markdown layout, so a half-typed
/// fence reads as text until its closing fence arrives.
pub(crate) fn clip_at<'a>(text: &'a str, state: &Reveal, now: u64, level: MotionLevel) -> &'a str {
    clip(text, visible_len(state, now, text.chars().count(), level))
}

/// Ramp the newest revealed cells from `text_muted` to their ink. Walks the
/// body's cells from the end — cells stand in for characters, close enough
/// for a dozen of them — and stops at the first one already at full ink.
pub(crate) fn glow_tail(
    lines: &mut [CardLine],
    state: &Reveal,
    now: u64,
    total: usize,
    level: MotionLevel,
) {
    if level == MotionLevel::Off {
        return;
    }
    let visible = visible_len(state, now, total, level);
    let muted = crew_theme::theme().text_muted;
    let cells = lines.iter_mut().rev().flat_map(|l| l.iter_mut().rev());
    for (k, cell) in cells.take(GLOW_CHARS.min(visible)).enumerate() {
        let t = char_mix(
            char_age_ms(state, now, visible - 1 - k, total, level),
            level,
        );
        if t >= 1.0 {
            break;
        }
        cell.fg = crate::anim::lerp_rgb(muted, cell.fg, t);
    }
}

/// Put a pulsing block on the end of a streaming card's last line — after the
/// last revealed character: the one unambiguous sign that text is still
/// arriving, as distinct from a reply that simply ended mid-sentence. It
/// pulses between the muted and accent colours rather than blinking on and
/// off: a caret that vanishes half the time reads like the text stopped.
pub(crate) fn push_caret(lines: &mut [CardLine], now_ms: u64, cols: usize) {
    let Some(last) = lines.last_mut() else { return };
    if last.len() >= cols {
        return;
    }
    let t = match crate::motion::level() {
        MotionLevel::Off => 1.0,
        _ => crate::anim::tri(now_ms, CARET_MS),
    };
    let th = crew_theme::theme();
    let fg = crate::anim::lerp_rgb(th.text_muted, crate::palette::accent(), t);
    last.push(crate::chatbody::plain('\u{258c}', fg, false));
}

/// A card's reveal state, addressed the way the cards are: by the agent's
/// `stream_key` while provisional (`ts: None` — one card per agent), by key
/// AND the settled `Message`'s stamp once it has landed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CardReveal {
    pub(crate) key: String,
    pub(crate) ts: Option<String>,
    pub(crate) state: Reveal,
}

/// The reveal state for card `m` (`streaming` = it sits past
/// `View::streaming_from`), if it is still being typed out.
pub(crate) fn find<'a>(rs: &'a [CardReveal], m: &Message, streaming: bool) -> Option<&'a Reveal> {
    let key = crate::chatflow::stream_key(&m.sender);
    rs.iter()
        .find(|r| r.key == key && r.ts.as_deref() == (!streaming).then_some(m.ts.as_str()))
        .map(|r| &r.state)
}

#[cfg(test)]
#[path = "chatreveal_tests.rs"]
mod tests;
