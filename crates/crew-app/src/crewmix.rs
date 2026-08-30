//! The crew mix: the PANES section's one-glance answer to *what is the crew
//! doing right now* — one chip per pane, on a row per state, with the state's
//! name and count beside it.
//!
//! It replaced a donut, and the donut replaced a sparkline. The donut could
//! say what the sparkline could not (three states, not one), but it answered
//! with an ANGLE, and the two things a crew is actually asked are "how many"
//! and "doing what" — both of them counts. A ring of one pane is a solid
//! disc, which is a large black circle spending three rows and seven columns
//! to say "1". A chip per pane says it by being one chip, stays exactly
//! countable to a dozen, and gives the three states a common baseline to be
//! compared along, which a pie never has.
//!
//! The pulse history did not go away: it is drawn as a faint fill behind the
//! legend, so the chips say the present and the wash behind them says the
//! last minute.
use crew_render::{CellView, Paint};

use crate::palette::accent;
use crate::panelist::PaneRow;
use crate::plot::Canvas;

/// Rows the block occupies, under the PANES header.
pub const ROWS: u16 = 3;

/// First column of the chip gutter — one column of margin, matching the rest
/// of the nav's indent.
const CHIP_COL: f32 = 1.0;
/// Columns the chips may claim. Six is what a docked nav can spare beside a
/// label and a count, and a crew of six is already a busy screen.
const CHIP_COLS: usize = 6;
/// Chip side and pitch, in canvas units (one unit = one cell width). The gap
/// is what makes them countable: chips that touch read as a bar.
const CHIP: f32 = 0.62;
const PITCH: f32 = 1.0;
/// Corner radius — a square with the corners taken off, so a chip reads as a
/// tile rather than a dot (a dot is what the legend swatch used to be, and
/// two round marks in one block said they meant the same thing).
const CHIP_R: f32 = 0.16;
/// First column of the legend — clear of the chips, with a column of air.
const LEGEND_COL: u16 = CHIP_COL as u16 + CHIP_COLS as u16 + 1;
/// Columns the legend claims right of its text column: the longest label
/// ("working") plus air and a count wide enough for any crew.
const BLOCK_W: u16 = 12;
/// Where the pulse backdrop starts: past the chips, so the two never overlap.
const WASH_X: f32 = CHIP_COL + CHIP_COLS as f32 * PITCH + 0.4;
/// How the crew divides up right now. `waiting` is a pane that has raised a
/// marker for you (a bell, a finished command); `working` is one doing
/// background work; everything else is idle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Mix {
    pub waiting: usize,
    pub working: usize,
    pub idle: usize,
}

impl Mix {
    pub fn total(&self) -> usize {
        self.waiting + self.working + self.idle
    }
}

/// Sort the open panes into the three states. Attention outranks busy: a pane
/// that wants you is *waiting on you*, whatever else it is doing — the same
/// precedence the pane row's own marker slot uses.
pub fn mix(panes: &[PaneRow]) -> Mix {
    let mut m = Mix::default();
    for p in panes {
        if p.attention.is_some() {
            m.waiting += 1;
        } else if p.busy {
            m.working += 1;
        } else {
            m.idle += 1;
        }
    }
    m
}

/// `(label, count, colour)` per legend line, in ring order.
fn entries(m: &Mix) -> [(&'static str, usize, (u8, u8, u8)); 3] {
    let t = crew_theme::theme();
    [
        ("working", m.working, accent()),
        ("waiting", m.waiting, t.bell),
        ("idle", m.idle, t.text_muted),
    ]
}

/// The block's text: one legend line per state, its count right-aligned.
/// `row0` is the row the block starts on inside the card.
///
/// The total no longer has a place of its own here. It used to sit in the
/// donut's hole, which is a hole this widget does not have; it is the sum of
/// three numbers already on screen, and the section's own rule carries it
/// (`PANES 8`) for the glance that wants it without reading three rows.
pub fn cells(m: &Mix, cols: u16, row0: u16) -> Vec<CellView> {
    let t = crew_theme::theme();
    let mut out = Vec::new();
    if cols <= LEGEND_COL + 4 {
        return out; // too narrow for a legend: the chips alone still read
    }
    let text_col = LEGEND_COL;
    for (k, (label, n, fg)) in entries(m).into_iter().enumerate() {
        let row = row0 + k as u16;
        // A category with no members is dimmed rather than dropped: the legend
        // is a fixed key, and a key that reorders itself as the crew changes
        // is harder to read than one that stays put.
        let fg = if n == 0 { t.text_muted } else { fg };
        let count = n.to_string();
        // The counts right-align, but to the *block's* right edge, not the
        // nav's: dragged wide, "working" sat at column 9 and its 2 at column
        // 35, and a key with twenty columns of nothing between the label and
        // the number stops reading as a pair.
        let right = cols.min(text_col + BLOCK_W);
        let cx = right.saturating_sub(1 + count.chars().count() as u16);
        // The count is the reading, so it is placed first and the label gets
        // what is left, with a column of air between them — and it ellipsizes,
        // because a narrow nav used to read `workin 2`, which is a word that
        // does not exist sitting flush against a number.
        crate::navtext::put_at(&mut out, label, text_col, row, cx.saturating_sub(1), fg);
        for (i, ch) in count.chars().enumerate() {
            out.push(cell(cx + i as u16, row, ch, fg, t.page_bg));
        }
    }
    out
}

/// How many chips a state of `n` panes draws, and whether the last one stands
/// for more than itself.
///
/// Past [`CHIP_COLS`] the gutter is full, so the final chip becomes an
/// overflow mark rather than the row silently under-counting: the number
/// beside it is exact, and the chips stop claiming to be.
fn chips(n: usize) -> (usize, bool) {
    match n > CHIP_COLS {
        true => (CHIP_COLS, true),
        false => (n, false),
    }
}

/// The block's drawing: the pulse wash, the ring, and the legend swatches.
/// `pulse` is the busy-pane history the old sparkline used.
pub fn paint(
    m: &Mix,
    cols: u16,
    row0: u16,
    aspect: f32,
    pulse: &crate::spark::History,
) -> Vec<Paint> {
    if cols == 0 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let mut c = Canvas::new(cols, ROWS, aspect);
    let (w, h) = c.size();

    // The last minute of crew workload, behind the legend: quiet enough to
    // read through (half the chart's own alpha — at full strength it fights
    // the text it sits under), present enough to see the shape of a swarm
    // that has since finished.
    let peak = pulse.peak(cols as usize).max(1);
    let samples: Vec<f32> = pulse
        .tail(cols as usize * 2)
        .into_iter()
        .map(|v| (v as f32 / peak as f32).clamp(0.0, 1.0))
        .collect();
    if !samples.is_empty() {
        let mut back = Canvas::new(cols, ROWS, aspect);
        // Fill only, and starting clear of the ring. With its stroke on, the
        // curve ran a faint line straight through "working" and "waiting" —
        // a scribble across the legend, not a backdrop behind it — and its
        // left end crossed the chips, which are the marks in the block the
        // eye is meant to land on.
        crate::plot::area::draw_styled(
            &mut back,
            (WASH_X, 0.0, (w - WASH_X).max(0.0), h),
            &samples,
            accent(),
            crate::plot::area::Style::wash(),
        );
        for p in back.paint() {
            c.rect(p.x, p.y * aspect, p.w, p.h * aspect, p.color, p.alpha * 0.5);
        }
    }

    // The chips go on their own, finer canvas, composited over the wash by
    // being emitted after it.
    let mut fg = Canvas::new(cols, ROWS, aspect);
    for (k, (_, n, col)) in entries(m).into_iter().enumerate() {
        // Vertically centred in the row, and square: the canvas is
        // aspect-corrected, so one unit is one cell WIDTH in both axes and a
        // chip given the same extent twice comes out square on screen.
        let cy = (k as f32 + 0.5) * aspect - CHIP * 0.5;
        let (count, over) = chips(n);
        for i in 0..count {
            let cx = CHIP_COL + i as f32 * PITCH;
            // An empty state still shows its baseline: one hollow chip, so
            // the three rows share a left edge to be compared along and a
            // crew with nothing waiting does not look like a missing row.
            let last = over && i + 1 == count;
            let colour = if last { t.ink } else { col };
            fg.fill_sdf((cx, cy, CHIP, CHIP), colour, 1.0, move |px, py| {
                crate::plot::sdf::round_box((px, py), cx, cy, CHIP, CHIP, CHIP_R)
            });
        }
        if count == 0 {
            // The empty marker: the chip's outline only, in the border
            // colour — present enough to hold the row's left edge, quiet
            // enough that nobody counts it as a pane.
            let cx = CHIP_COL;
            fg.fill_sdf((cx, cy, CHIP, CHIP), t.border_normal, 1.0, move |px, py| {
                crate::plot::sdf::round_box((px, py), cx, cy, CHIP, CHIP, CHIP_R).abs() - 0.06
            });
        }
    }
    c.paint()
        .into_iter()
        .chain(fg.paint())
        .map(|p| p.shifted(0.0, f32::from(row0)))
        .collect()
}

fn cell(col: u16, row: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row,
        c,
        fg,
        bg,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{cells, mix, paint, Mix, ROWS};
    use crate::panelist::PaneRow;

    fn pane(busy: bool, attention: bool) -> PaneRow {
        PaneRow {
            index: 1,
            title: "x".into(),
            focused: false,
            activity: false,
            minimized: false,
            attention: attention.then_some(('!', true)),
            busy,
            unread: 0,
            hovered: false,
        }
    }

    #[test]
    fn a_pane_that_wants_you_is_waiting_whatever_else_it_is_doing() {
        // Same precedence as the row's own marker slot: attention outranks
        // busy, so a swarm that has raised a bell is counted once, as waiting.
        let m = mix(&[pane(true, true), pane(true, false), pane(false, false)]);
        assert_eq!(
            m,
            Mix {
                waiting: 1,
                working: 1,
                idle: 1
            }
        );
        assert_eq!(m.total(), 3);
    }

    /// One row per state, its own count on the end of it. The total is not
    /// here: it rides the section rule (`PANES 13`), because it is the sum of
    /// three numbers already on screen and the block has no hole to put it in
    /// any more.
    #[test]
    fn each_state_gets_a_row_and_its_own_count() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 1,
            working: 12,
            idle: 0,
        };
        let out = cells(&m, 24, 0);
        let row = |r: u16| -> String {
            let mut v: Vec<_> = out.iter().filter(|c| c.row == r).collect();
            v.sort_by_key(|c| c.col);
            v.iter().map(|c| c.c).collect()
        };
        assert!(row(0).contains("working") && row(0).ends_with("12"));
        assert!(row(1).contains("waiting") && row(1).ends_with('1'));
        assert!(row(2).contains("idle") && row(2).ends_with('0'));
    }

    /// The chips are countable up to the gutter, and past it the last one
    /// says so instead of the row quietly under-counting. The number beside
    /// them stays exact either way.
    #[test]
    fn chips_count_the_panes_and_mark_their_own_overflow() {
        assert_eq!(super::chips(0), (0, false));
        assert_eq!(super::chips(1), (1, false));
        assert_eq!(super::chips(super::CHIP_COLS), (super::CHIP_COLS, false));
        assert_eq!(super::chips(super::CHIP_COLS + 1), (super::CHIP_COLS, true));
        assert_eq!(super::chips(400), (super::CHIP_COLS, true));
    }

    #[test]
    fn an_empty_category_is_dimmed_not_dropped() {
        let _g = crate::app::theme_test_guard();
        let t = crew_theme::theme();
        let out = cells(
            &Mix {
                waiting: 0,
                working: 2,
                idle: 1,
            },
            24,
            0,
        );
        // The key stays in place — a legend that reorders itself as the crew
        // changes is harder to read than one that does not.
        let waiting: Vec<_> = out.iter().filter(|c| c.row == 1 && c.c == 'w').collect();
        assert!(!waiting.is_empty(), "the waiting line is still drawn");
        assert!(waiting.iter().all(|c| c.fg == t.text_muted));
    }

    /// A legend label too wide for the nav ends in `…`, one column clear of
    /// its count. `workin 2` — a word that does not exist, flush against a
    /// number — is what the narrow end of the resize range used to show.
    #[test]
    fn a_narrow_legend_ellipsizes_and_keeps_its_air() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 0,
            working: 2,
            idle: 1,
        };
        // One column narrower than "working" needs beside its count, now
        // that the chips claim the gutter the ring used to.
        let cells = cells(&m, 17, 0);
        let row: String = {
            let mut v: Vec<_> = cells.iter().filter(|c| c.row == 0).collect();
            v.sort_by_key(|c| c.col);
            let mut out = String::new();
            let mut at = v[0].col;
            for c in v {
                for _ in at..c.col {
                    out.push(' ');
                }
                out.push(c.c);
                at = c.col + 1;
            }
            out
        };
        assert!(row.ends_with(" 2"), "the count is whole: {row:?}");
        assert!(
            row.contains('\u{2026}'),
            "and the label says it was cut: {row:?}"
        );
        assert!(!row.contains("\u{2026}2"), "with air between them: {row:?}");
    }

    #[test]
    fn a_narrow_sidebar_keeps_the_chips_and_drops_the_legend() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 0,
            working: 1,
            idle: 1,
        };
        let text: String = cells(&m, 9, 0).iter().map(|c| c.c).collect();
        assert_eq!(text, "", "no room for a label and a count");
        // The chips themselves are still drawn: the widget degrades, it does
        // not disappear — and a column of marks still says how many panes
        // there are and what they are doing.
        let hist = crate::spark::History::new(8);
        assert!(!paint(&m, 9, 0, 2.0, &hist).is_empty());
    }

    #[test]
    fn the_ring_is_drawn_inside_the_rows_the_block_reserved() {
        let _g = crate::app::theme_test_guard();
        let m = Mix {
            waiting: 1,
            working: 1,
            idle: 1,
        };
        let hist = crate::spark::History::new(8);
        let p = paint(&m, 24, 7, 2.0, &hist);
        assert!(!p.is_empty());
        for r in &p {
            assert!(r.y >= 7.0, "starts at the block's first row: {r:?}");
            assert!(
                r.y + r.h <= 7.0 + f32::from(ROWS) + 1e-3,
                "and stays inside it: {r:?}"
            );
        }
    }

    /// The wash is a backdrop, and a backdrop that crosses the chips is a
    /// smudge on the marks the block is about. Nothing it draws may reach the
    /// gutter, and it may not draw a curve at all.
    #[test]
    fn the_pulse_wash_never_reaches_the_chips() {
        let _g = crate::app::theme_test_guard();
        let mut hist = crate::spark::History::new(64);
        for v in 0..40 {
            hist.push(v % 5);
        }
        let m = Mix::default(); // no ring of its own: every accent quad is wash
        let p = paint(&m, 24, 0, 2.0, &hist);
        let wash: Vec<_> = p
            .iter()
            .filter(|r| r.color == crate::palette::accent())
            .collect();
        assert!(!wash.is_empty(), "there is a wash to test");
        let gutter_right = super::CHIP_COL + super::CHIP_COLS as f32 * super::PITCH;
        assert!(
            wash.iter().all(|r| r.x >= gutter_right),
            "wash crosses the chips (gutter ends {gutter_right}): {wash:?}"
        );
    }

    #[test]
    fn the_pulse_wash_stays_quiet_enough_to_read_through() {
        let _g = crate::app::theme_test_guard();
        let m = Mix::default();
        let mut hist = crate::spark::History::new(64);
        for v in 0..40 {
            hist.push(v % 5);
        }
        let p = paint(&m, 24, 0, 2.0, &hist);
        // The backdrop is texture, not a second chart competing with the
        // legend written over it.
        let loud = p.iter().filter(|r| r.alpha > 0.4).count();
        let quiet = p.iter().filter(|r| r.alpha <= 0.4).count();
        assert!(quiet > loud, "wash {quiet} quiet vs {loud} loud rectangles");
    }
}
