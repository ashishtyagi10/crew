//! A settled card's arrival: the newest card slides up into place from one
//! row below while it fades in (`chatcard::fade_t`), so a reply LANDS rather
//! than appears. The slide is a row offset the renderer applies to that
//! card's lines only — and only while the pane is at its live bottom, so
//! nothing above it ever jumps — clipped at the window's last row.
//!
//! Also where the transcript's lines are placed for drawing ([`place`]):
//! the glide has to see the card spans `card_lines_spanned` recorded, and
//! doing the windowing here keeps `chatmsgs` and `chatplace` under the cap.
use crate::chatbody::CardLine;
use crate::chatlayout::Message;
use crate::chatmsgs::View;
use crate::motion::MotionLevel;

/// How long the slide takes, at full motion.
pub(crate) const GLIDE_MS: u64 = 220;

/// Eased progress of the glide for a card stamped `ts` (epoch ms): 0 just
/// landed, 1 in place. Motion Off, an unparseable stamp and the counting pass
/// (`now_ms == 0`) are all 1 — the card is simply drawn where it belongs.
pub(crate) fn glide_t(ts: &str, now_ms: u64, level: MotionLevel) -> f32 {
    let dur = level.scale_ms(GLIDE_MS);
    if now_ms == 0 || dur == 0 {
        return 1.0;
    }
    let Ok(ts) = ts.parse::<u64>() else {
        return 1.0;
    };
    crate::ease::out_cubic(now_ms.saturating_sub(ts) as f32 / dur as f32)
}

/// Rows below its resting place the card sits at `now_ms`: one until the
/// glide has run its [`GLIDE_MS`], then zero. A one-row slide has a single
/// intermediate state; the ease in [`glide_t`] is what the fade beside it
/// shares, not a fractional row.
pub(crate) fn offset_rows(ts: &str, now_ms: u64, level: MotionLevel) -> u16 {
    u16::from(glide_t(ts, now_ms, level) < 1.0)
}

/// The glide offset of the last card, or 0 when it must not glide: the view
/// is scrolled up (rows above would move), the card is still streaming
/// (it is not new — it has been on screen typing), or it was typed out on
/// screen before it settled (`Reveal::settled`, the same rule the fade uses).
fn last_card_offset(messages: &[&Message], view: View<'_>, scroll: usize, now_ms: u64) -> u16 {
    let Some((i, m)) = messages.iter().enumerate().next_back() else {
        return 0;
    };
    if scroll > 0 || i >= view.streaming_from {
        return 0;
    }
    if crate::chatreveal::find(view.reveals, m, false).is_some_and(|r| r.settled) {
        return 0;
    }
    offset_rows(&m.ts, now_ms, crate::motion::level())
}

/// The scroll-windowed card lines of `messages` in a `rows`-row window from
/// `top_row`, each tagged with its absolute row — `chatplace::window` plus
/// the arrival glide on the last card. `now_ms` is the fade/glide clock
/// (epoch ms, like `Message::ts`).
pub(crate) fn place(
    messages: &[&Message],
    cols: u16,
    rows: u16,
    top_row: u16,
    scroll: usize,
    view: View<'_>,
    now_ms: u64,
) -> Vec<(u16, CardLine)> {
    let (lines, spans) = crate::chatmsgs::card_lines_spanned(messages, cols as usize, now_ms, view);
    let total = lines.len();
    let mut placed = crate::chatplace::window(lines, rows, top_row, scroll);
    let offset = last_card_offset(messages, view, scroll, now_ms);
    let Some(span) = spans.last().filter(|_| offset > 0 && rows > 0) else {
        return placed;
    };
    // At the live bottom the window starts `total - rows` lines in (or at 0
    // with slack above), so placed entry `k` is line `first + k`. The card
    // and everything after it (tool orphans) shift; the rows that fall past
    // the window's last row are clipped — that is the card's bottom edge.
    let first = total.saturating_sub(rows as usize);
    let last_row = top_row + rows - 1;
    for (k, (row, _)) in placed.iter_mut().enumerate() {
        if first + k >= span.start {
            *row += offset;
        }
    }
    placed.retain(|(row, _)| *row <= last_row);
    placed
}

#[cfg(test)]
#[path = "chatglide_tests.rs"]
mod tests;
