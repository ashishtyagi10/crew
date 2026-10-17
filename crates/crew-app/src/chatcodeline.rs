//! The code line: one row above the composer saying agent smith is at work,
//! in the Matrix's own voice — a word decoding out of flickering glyphs, a
//! streak of code running after it, and the turn's clock on the right.
//!
//! A ROW, not a patch. The busy rain tried on 2026-07-13 sat in the card's
//! corner, and the border layer drew it over the composer's prompt. A row
//! granted by `chatplace::grants` is how the swarm's bar (`chatprog`) and the
//! queued indicator stay off everything else, and this line shares the bar's
//! row: a swarm run keeps it for the bar, and this one stands down.
//!
//! It moves only while work does. Every broker event that is not a heartbeat
//! (`chatwatch` stamps it) lights the line for a beat; after [`STALL_MS`] of
//! nothing it slows and dims and holds its word, so a stuck turn looks stuck
//! instead of spinning forever. No tool names and no counts: a running tool
//! adds a second streak, and that is all it says.
use crew_render::CellView;

use crate::anim::lerp_rgb;
use crate::charrain::{hash, GLYPHS, HEADS};
use crate::chat::ChatPane;
use crate::chatcodeword::{slot, word, Slot};
use crate::motion::MotionLevel;

/// Left inset, matching the bar and the queued indicator.
const INSET: u16 = 1;
/// Narrowest pane the line is drawn in: a word, a streak and a clock.
pub(crate) const MIN_COLS: u16 = 24;
/// How long one event keeps the line lit.
const LIT_MS: u64 = 1_200;
/// Nothing new for this long and the line reads stalled.
pub(crate) const STALL_MS: u64 = 8_000;
/// Glyph re-roll and streak step while lively; stalled runs this much slower.
const FLICK_MS: u64 = 90;
const STEP_MS: u64 = 55;
const STALL_SLOW: u64 = 4;
/// The streak's field after the word, and the streak's own length.
const STREAM_MAX: u16 = 24;
const STREAM_MIN: u16 = 6;
const TRAIL: u16 = 7;
/// Streak cells from this depth on are dots, as in the welcome's rain.
const DOTS_FROM: u16 = 3;

/// Everything one frame of the line depends on, so a test can stand anywhere
/// in a turn.
pub(crate) struct Frame {
    pub cols: u16,
    pub row: u16,
    /// When the turn began, and its last sign of work (animation clock).
    pub since: u64,
    pub last_work: u64,
    /// A tool is running: a second streak.
    pub tool: bool,
    pub level: MotionLevel,
}

/// Rows the line claims: 1 while the pane is on a turn of its own and wide
/// enough. Not during a swarm run (its bar has the row) and not while a
/// question waits on you, when nothing is being worked on.
pub(crate) fn code_rows(pane: &ChatPane, cols: u16) -> u16 {
    let own = pane.is_busy() && pane.swarm.is_none() && pane.asking.is_empty();
    u16::from(own && cols >= MIN_COLS)
}

/// The live rows directly above the composer: a swarm run's bar, or the code
/// line with the agent's plan over it. One grant, because the two never show
/// at once.
pub(crate) fn live_rows(pane: &ChatPane, cols: u16) -> u16 {
    crate::chatprog::progress_rows(pane, cols).max(crate::chatchecklist::rows(pane, cols))
}

/// The line at `row` on the animation clock, or nothing when it has no row.
pub(crate) fn line_cells(pane: &ChatPane, cols: u16, row: u16) -> Vec<CellView> {
    if code_rows(pane, cols) == 0 {
        return Vec::new();
    }
    let now = crate::anim::now_ms();
    let (since, last_work) = pane.work_clock(now);
    let (tool, level) = (pane.tools_running(), crate::motion::level());
    cells_at(
        &Frame {
            cols,
            row,
            since,
            last_work,
            tool,
            level,
        },
        now,
    )
}

/// One frame of the line at `now`.
pub(crate) fn cells_at(f: &Frame, now: u64) -> Vec<CellView> {
    let t = crew_theme::theme();
    let (bg, mut out) = (t.page_bg, Vec::new());
    if f.cols < MIN_COLS {
        return out;
    }
    let clock = crate::runclock::ladder(now.saturating_sub(f.since) / 1000);
    let word = word(f.since);
    let mut put = |col: u16, c: char, fg: (u8, u8, u8), bold: bool| {
        let (row, italic) = (f.row, false);
        out.push(CellView {
            col,
            row,
            c,
            fg,
            bg,
            bold,
            italic,
            ..Default::default()
        });
    };
    if f.level == MotionLevel::Off {
        // Still, and every word of it said: who, what, how long.
        let text = format!("smith \u{b7} {word} \u{b7} {clock}");
        let lit = INSET + 8..INSET + 8 + word.chars().count() as u16;
        for (col, c) in (INSET..f.cols - 1).zip(text.chars()) {
            let on = lit.contains(&col);
            put(col, c, if on { t.ink } else { t.text_muted }, false);
        }
        return out;
    }
    let quiet = now.saturating_sub(f.last_work);
    let stalled = quiet > STALL_MS;
    let lit = 1.0 - (quiet as f32 / LIT_MS as f32).min(1.0);
    let slow = if stalled { STALL_SLOW } else { 1 };
    let ms = |base: u64| crate::shimmer::period(base, f.level);
    let dim = |c| if stalled { lerp_rgb(c, bg, 0.45) } else { c };
    // The welcome's rain colours; heads in the page's ink, which is white
    // on the white-text glass.
    let rain = lerp_rgb(t.text_muted, bg, 0.35);
    let (head, glyph) = (dim(t.ink), dim(lerp_rgb(rain, t.text_muted, lit)));
    let letter = dim(lerp_rgb(t.text_muted, t.ink, 0.35 + 0.65 * lit));
    let flick = now / (ms(FLICK_MS) * slow).max(1);
    let pick = |i: u64, n: usize| GLYPHS[(hash(i, flick) % n as u64) as usize] as char;

    let n = word.chars().count() as u64;
    let at = now.saturating_sub(f.since) % crate::chatcodeword::cycle(n, ms);
    for (i, c) in word.chars().enumerate() {
        let col = INSET + i as u16;
        // A stalled line holds its word: nothing is being decoded.
        let shown = (!stalled).then(|| slot(i as u64, n, at, f.since, ms));
        match shown.unwrap_or(Slot::Letter) {
            Slot::Letter if c == ' ' => {}
            Slot::Letter => put(col, c, letter, false),
            Slot::Head => put(col, pick(i as u64, HEADS), head, true),
            Slot::Glyph => put(col, pick(i as u64, GLYPHS.len()), glyph, false),
        }
    }

    let clock_w = clock.chars().count() as u16;
    // Ends a column short of the pane's last, as the header's `●` does.
    let clock_col = f.cols - 1 - clock_w;
    for (k, c) in clock.chars().enumerate() {
        put(clock_col + k as u16, c, dim(t.text_muted), false);
    }

    let x0 = INSET + n as u16 + 2;
    let fw = clock_col
        .saturating_sub(2)
        .saturating_sub(x0)
        .min(STREAM_MAX);
    if fw < STREAM_MIN {
        return out;
    }
    let period = (fw + TRAIL) as u64;
    let pos = now / (ms(STEP_MS) * slow).max(1);
    let tail = lerp_rgb(glyph, bg, 0.45);
    let heads = [Some(pos), f.tool.then_some(pos + period / 2)];
    for h in heads.into_iter().flatten().map(|p| (p % period) as i64) {
        for d in 0..TRAIL {
            let x = h - d as i64;
            if x < 0 || x >= fw as i64 {
                continue;
            }
            let fg = lerp_rgb(tail, head, 1.0 - d as f32 / TRAIL as f32);
            let c = match d {
                0 => pick(100 + x as u64, HEADS),
                d if d >= DOTS_FROM => '\u{b7}',
                _ => pick(100 + x as u64, GLYPHS.len()),
            };
            put(x0 + x as u16, c, fg, d == 0);
        }
    }
    out
}

#[cfg(test)]
#[path = "chatcodeline_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "chatcodelinerow_tests.rs"]
mod row_tests;
