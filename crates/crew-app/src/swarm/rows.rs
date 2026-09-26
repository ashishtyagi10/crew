//! What fits on a swarm row.
//!
//! The pane's list is laid out by width, and a tile can be thirty columns
//! wide. Every string here degrades on purpose — a HUD that drops its cost
//! before it cuts a number in half, a title that is cut with a mark rather
//! than mid-word, a tail that goes missing before it goes meaningless —
//! because a number sliced by the frame reads as a different number.
use crate::chatwidth::{clip_w, str_w};

/// Columns the state glyph takes at the start of a task row: ` ● `.
pub const GLYPH_COLS: usize = 3;

/// The HUD, in the widest form that fits `cols`: with the cost, without it,
/// then the glyph form the list rows already use.
///
/// A count of zero is not said: `0 failed` sat on every healthy run and
/// `0 live` on every finished one (the `/todo` bands drop theirs the same
/// way). `done` stays while nothing else is left to say.
pub fn hud_text(live: usize, done: usize, failed: usize, micros_usd: u64, cols: u16) -> String {
    // Counts before their nouns, parted by `·` like every other reading in
    // crew: `live:1 done:3 failed:1 cost:$0.0420` read as a debug print.
    let cost = crate::usagepane::money(micros_usd);
    let said = |n: usize, keep: bool| n > 0 || keep;
    let keep_done = live == 0 && failed == 0;
    let words: Vec<String> = [
        (live, "live", false),
        (done, "done", keep_done),
        (failed, "failed", false),
    ]
    .into_iter()
    .filter(|&(n, _, keep)| said(n, keep))
    .map(|(n, w, _)| format!("{n} {w}"))
    .collect();
    let glyphs: String = [
        (live, '\u{25cf}', false),
        (done, '\u{2713}', keep_done),
        (failed, '\u{2717}', false),
    ]
    .into_iter()
    .filter(|&(n, _, keep)| said(n, keep))
    .map(|(n, g, _)| format!(" {g}{n}"))
    .collect();
    let counts = words.join(" \u{b7} ");
    let forms = [
        format!(" {counts} \u{b7} {cost}"),
        format!(" {counts}"),
        glyphs,
    ];
    let cols = cols as usize;
    let last = forms[forms.len() - 1].clone();
    forms
        .into_iter()
        .find(|f| str_w(f) <= cols)
        .unwrap_or_else(|| clip_w(&last, cols))
}

/// A task row's title and ` — tail`, fitted after the glyph: the title keeps
/// what it can and marks its cut; the tail takes the remainder, or nothing
/// when the remainder could not hold a word.
pub fn task_row(title: &str, tail: &str, cols: u16) -> (String, String) {
    let room = (cols as usize).saturating_sub(GLYPH_COLS);
    let title = clip_w(title, room);
    let left = room - str_w(&title);
    // ` — ` is three columns; below three more there is no word to show.
    let tail = if tail.is_empty() || left < 6 {
        String::new()
    } else {
        clip_w(&format!(" \u{2014} {tail}"), left)
    };
    (title, tail)
}

/// How many of `n` tasks a list `rows` tall names, under a HUD row and —
/// when they do not all fit — above one row kept for `… +N more`.
pub fn shown(n: usize, rows: u16) -> usize {
    let avail = rows.saturating_sub(1) as usize;
    if n > avail {
        avail.saturating_sub(1)
    } else {
        n
    }
}

#[cfg(test)]
#[path = "rows_tests.rs"]
mod tests;
