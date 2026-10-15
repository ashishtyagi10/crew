use super::{count, position};
use crate::panecard::Bar;
use crew_render::CellView;

/// The rows the DRAWN thumb touches on the right border of a `cols`×`rows`
/// card. The thumb is a paint rectangle at a fractional position now, so the
/// same questions these tests always asked — where is it, how long is it —
/// are answered by which rows its rectangle covers.
fn thumb_rows_of(b: &Bar, cols: u16, rows: u16) -> Vec<u16> {
    let paint = crate::cardpaint::card_paint(cols, rows, b, (8.0, 16.0), 0);
    let mut out: Vec<u16> = Vec::new();
    for p in paint.iter().filter(|p| p.x >= f32::from(cols - 1) - 0.01) {
        let first = p.y.floor() as u16;
        let last = (p.y + p.h - 1e-3).floor() as u16;
        for r in first..=last {
            if !out.contains(&r) {
                out.push(r);
            }
        }
    }
    out.sort_unstable();
    out
}

fn bar(scroll: usize, total: usize) -> Bar<'static> {
    Bar {
        index: None,
        title: "sh",
        focused: false,
        scroll,
        total,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        assemble_t: 1.0,
        focus_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        pinned: false,
        at_cmd: None,
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread: 0,
        doc: crate::panehome::Doc::No,
    }
}

/// The thumb rows for a card scrolled `scroll` back in a `total`-line buffer.
fn thumb_rows(cols: u16, rows: u16, scroll: usize, total: usize) -> Vec<u16> {
    thumb_rows_of(&bar(scroll, total), cols, rows)
}

#[test]
fn nothing_is_drawn_at_the_bottom_of_the_buffer() {
    assert!(thumb_rows(40, 20, 0, 5_000).is_empty());
}

#[test]
fn nothing_is_drawn_when_there_is_no_scrollback_to_speak_of() {
    // 18 visible rows, 18 lines in total: the window IS the buffer.
    assert!(thumb_rows(40, 20, 3, 18).is_empty());
}

#[test]
fn a_short_card_draws_no_gutter() {
    assert!(thumb_rows(40, 4, 100, 5_000).is_empty());
}

/// The thumb stays inside the frame: never on a corner, never past the
/// bottom border.
#[test]
fn the_thumb_stays_between_the_corners() {
    for scroll in [1, 40, 400, 4_000, 4_982] {
        let rows = thumb_rows(40, 20, scroll, 5_000);
        assert!(!rows.is_empty(), "scroll {scroll} draws something");
        assert!(
            rows.iter().all(|&r| (1..19).contains(&r)),
            "scroll {scroll} drew on {rows:?}, outside rows 1..19"
        );
    }
}

/// It is a *position* indicator, not just a marker: scrolling further back
/// must move it up the border, and the deepest scroll must reach the top.
#[test]
fn scrolling_back_walks_the_thumb_up_the_border() {
    let near = thumb_rows(40, 20, 10, 5_000);
    let far = thumb_rows(40, 20, 4_000, 5_000);
    assert!(
        far[0] < near[0],
        "further back ({far:?}) should sit above nearer ({near:?})"
    );
    let deepest = thumb_rows(40, 20, 5_000 - 18, 5_000);
    assert_eq!(deepest[0], 1, "the top of the buffer reaches the top row");
}

/// A big buffer gets a small thumb — that is the "how much is there" half of
/// what the indicator exists to say.
#[test]
fn the_thumb_is_proportional_to_how_much_there_is() {
    let shallow = thumb_rows(40, 20, 5, 40).len();
    let deep = thumb_rows(40, 20, 5, 20_000).len();
    assert!(
        shallow > deep,
        "40 lines gave a {shallow}-cell thumb, 20 000 gave {deep}"
    );
    assert!(deep >= 1, "it never vanishes entirely");
}

#[test]
fn the_count_reports_the_next_free_column_and_writes_nothing_at_the_bottom() {
    let mut v: Vec<CellView> = Vec::new();
    assert_eq!(
        count(&mut v, 37, 2, (0, '⇡'), 0.5),
        37,
        "no scroll, no change"
    );
    assert!(v.is_empty());
    let next = count(&mut v, 37, 2, (12, '⇡'), 0.5);
    assert_eq!(v.len(), 3, "⇡12 is three glyphs");
    assert!(next < 35, "and the next glyph goes to its left");
}

/// A card too narrow for the label leaves the border alone rather than
/// writing over the legend.
#[test]
fn a_count_with_no_room_writes_nothing() {
    let mut v: Vec<CellView> = Vec::new();
    assert_eq!(count(&mut v, 1, 2, (4_000, '⇡'), 0.5), 1);
    assert!(v.is_empty());
}

/// Where the gutter sends you: the top of the interior is the top of the
/// buffer, the bottom is the live edge, and the middle is the middle.
#[test]
fn a_seek_to_the_top_of_the_gutter_reaches_the_top_of_the_buffer() {
    use super::offset_at;
    let (total, visible) = (5_000usize, 20usize);
    assert_eq!(offset_at(total, visible, 0.0), total - visible, "the top");
    assert_eq!(offset_at(total, visible, 1.0), 0, "the live edge");
    let mid = offset_at(total, visible, 0.5);
    assert!(
        mid.abs_diff((total - visible) / 2) <= 1,
        "halfway landed on {mid}"
    );
}

/// A drag past either end of the card, and a buffer that fits on screen,
/// must not produce an offset the grid would clamp away or panic on.
#[test]
fn a_seek_outside_the_gutter_or_with_no_history_is_bounded() {
    use super::offset_at;
    assert_eq!(offset_at(5_000, 20, -3.0), 4_980, "above the card");
    assert_eq!(offset_at(5_000, 20, 9.0), 0, "below it");
    assert_eq!(offset_at(20, 20, 0.0), 0, "the buffer fits on screen");
    assert_eq!(offset_at(5, 20, 0.0), 0, "and so does a shorter one");
}

/// The third reading of the scroll position: where you are in the buffer,
/// as a colour. Top of the scrollback is one end of the gradient, the live
/// bottom the other, and the walk between them is monotonic — a colour that
/// doubled back would say you had moved somewhere you had not.
#[test]
fn the_position_runs_end_to_end_and_never_doubles_back() {
    let (total, visible) = (1_000, 40);
    let range = total - visible;
    assert_eq!(position(total, visible, range), 0.0, "top of the buffer");
    assert_eq!(position(total, visible, 0), 1.0, "the live bottom");
    let mut prev = -1.0;
    for back in (0..=range).step_by(37) {
        let t = position(total, visible, range - back);
        assert!(t > prev, "position must climb: {t} !> {prev}");
        assert!((0.0..=1.0).contains(&t), "{t} is off the gradient");
        prev = t;
    }
}

/// Scrolled further back than there is buffer is still the top of the buffer,
/// not a colour off the end of the gradient.
#[test]
fn a_position_past_the_top_pins_to_the_top() {
    assert_eq!(position(100, 40, 9_999), 0.0);
}

/// A buffer with no history has no position to report. The midpoint keeps the
/// marker off both ends of the gradient rather than claiming you are at one.
#[test]
fn a_buffer_with_no_history_has_no_position() {
    assert_eq!(position(40, 40, 0), 0.5);
    assert_eq!(position(10, 40, 0), 0.5);
    assert_eq!(position(0, 0, 0), 0.5);
}

/// Both readings of the position — the `⇡N` count and the thumb — must wear
/// the SAME colour, or the border would be telling two stories about one
/// number. They are drawn by different functions, so this is the only thing
/// holding them together.
#[test]
fn both_readings_of_the_position_share_one_colour() {
    let _g = crate::app::theme_test_guard();
    let (total, visible) = (1_000usize, 8usize);
    let rows = (visible + 2) as u16;
    let b = bar(400, total);
    let thumb_fg = crate::cardpaint::card_paint(40, rows, &b, (8.0, 16.0), 0)
        .into_iter()
        .find(|p| p.x >= 39.0)
        .expect("a thumb should be drawn")
        .color;
    let mut count_cells = Vec::new();
    count(
        &mut count_cells,
        30,
        2,
        (b.scroll, '⇡'),
        position(total, visible, b.scroll),
    );
    let count_fg = count_cells.first().expect("a count should be drawn").fg;
    assert_eq!(thumb_fg, count_fg);
}

/// A document's gutter answers "where am I", which is a question you have at
/// the top of the file too — unlike a shell's, which only says how much is
/// behind you.
#[test]
fn a_document_draws_its_thumb_before_it_is_scrolled() {
    let _g = crate::app::theme_test_guard();
    let doc = Bar {
        scroll: 0,
        total: 400,
        doc: crate::panehome::Doc::File,
        ..bar(0, 400)
    };
    let shell = Bar {
        doc: crate::panehome::Doc::No,
        ..bar(0, 400)
    };
    let drawn = |b: &Bar| thumb_rows_of(b, 40, 20).len();
    assert!(drawn(&doc) > 0, "a document at the top shows no position");
    assert_eq!(drawn(&shell), 0, "a shell grew a permanent gutter");
}

fn pct(n: u8) -> Option<crew_term::Progress> {
    Some(crew_term::Progress {
        percent: Some(n),
        alarm: false,
    })
}

/// A program reporting progress gets a bar along the bottom border, filling
/// from the left in proportion to what it said.
#[test]
fn a_progress_report_fills_the_bottom_border() {
    let _g = crate::app::theme_test_guard();
    let bar_at = |p| Bar {
        progress: p,
        ..bar(0, 0)
    };
    // The bar is drawn now, so its reading is its WIDTH rather than a count
    // of cells — which is the point: 50% is half the border, not four glyphs.
    let bar_paint = |p| {
        crate::cardpaint::card_paint(42, 10, &bar_at(p), (8.0, 16.0), 0)
            .into_iter()
            .filter(|q| q.y >= 9.0)
            .collect::<Vec<_>>()
    };
    let width = |p| {
        let v = bar_paint(p);
        let lo = v.iter().map(|q| q.x).fold(f32::MAX, f32::min);
        let hi = v.iter().map(|q| q.x + q.w).fold(0.0f32, f32::max);
        if v.is_empty() {
            0.0
        } else {
            hi - lo
        }
    };
    assert!(bar_paint(None).is_empty(), "a quiet pane drew a bar");
    let half = bar_paint(pct(50));
    assert!(!half.is_empty());
    assert!(
        half.iter().all(|q| q.x >= 1.0 && q.x + q.w <= 41.0),
        "the bar overran the corners"
    );
    assert!((width(pct(50)) - 20.0).abs() < 0.3, "half the border");
    assert!((width(pct(99)) - 39.6).abs() < 0.3, "nearly all of it");
    // …and a FULL bar draws nothing. A bar at 100 is not a reading, it is a
    // line, and almost nothing clears its OSC 9;4 before exiting — so a
    // program that finished and kept running left a saturated stroke pinned
    // under the pane for the rest of the session with nothing saying what it
    // was. Every progress bar outside a terminal vanishes when it fills.
    assert!(
        bar_paint(pct(100)).is_empty(),
        "a full bar is a line, not a bar"
    );
    // The reading the glyph bar could not show at all: at 3% it drew one
    // whole cell — 2.5% of the border — or nothing.
    assert!(width(pct(3)) > 0.9, "3% is drawn");
    assert!(width(pct(3)) < width(pct(6)), "3% and 6% differ");
    assert!(bar_paint(pct(0)).is_empty(), "0% drew something");
}

/// An error or warning state is the same bar in the alarm colour: the number
/// still matters, and so does the fact that it went wrong.
#[test]
fn an_alarming_report_is_drawn_in_the_alarm_colour() {
    let _g = crate::app::theme_test_guard();
    let alarm = Bar {
        progress: Some(crew_term::Progress {
            percent: Some(60),
            alarm: true,
        }),
        ..bar(0, 0)
    };
    let bell = crew_theme::theme().bell;
    let v = crate::cardpaint::card_paint(42, 10, &alarm, (8.0, 16.0), 0);
    assert!(!v.is_empty());
    assert!(v.iter().all(|q| q.color == bell));
}

/// "Working, with no number" sweeps rather than filling — and it moves, or it
/// would read as a stuck bar at a random percentage.
#[test]
fn an_indeterminate_report_sweeps_instead_of_filling() {
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        progress: Some(crew_term::Progress {
            percent: None,
            alarm: false,
        }),
        ..bar(0, 0)
    };
    let at = |now: u64| {
        crate::cardpaint::card_paint(42, 10, &b, (8.0, 16.0), now)
            .into_iter()
            .map(|q| q.x)
            .fold(f32::MAX, f32::min)
    };
    assert!(at(0) < at(700), "the sweep does not move");
    let span = |now: u64| {
        let v = crate::cardpaint::card_paint(42, 10, &b, (8.0, 16.0), now);
        let lo = v.iter().map(|q| q.x).fold(f32::MAX, f32::min);
        let hi = v.iter().map(|q| q.x + q.w).fold(0.0f32, f32::max);
        hi - lo
    };
    assert!(span(700) < 20.0, "the sweep is a comet, not a fill");
    // It fades toward its tail — a block of constant brightness cannot say
    // which way it is going.
    let v = crate::cardpaint::card_paint(42, 10, &b, (8.0, 16.0), 700);
    let alphas: Vec<f32> = v.iter().map(|q| q.alpha).collect();
    let lo = alphas.iter().cloned().fold(f32::MAX, f32::min);
    let hi = alphas.iter().cloned().fold(0.0f32, f32::max);
    assert!(hi - lo > 0.3, "the sweep has a tail: {lo}..{hi}");
}

/// A card too small to hold a border bar draws none rather than a corner.
#[test]
fn a_tiny_card_draws_no_bar() {
    let _g = crate::app::theme_test_guard();
    let b = Bar {
        progress: pct(50),
        ..bar(0, 0)
    };
    assert!(crate::cardpaint::card_paint(3, 10, &b, (8.0, 16.0), 0).is_empty());
    assert!(crate::cardpaint::card_paint(42, 2, &b, (8.0, 16.0), 0).is_empty());
}
