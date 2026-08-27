use super::{count, position, thumb, MIN_ROWS};
use crate::panecard::Bar;
use crew_render::CellView;

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
    }
}

/// The thumb cells on the right border of a `cols`×`rows` card.
fn thumb_rows(cols: u16, rows: u16, scroll: usize, total: usize) -> Vec<u16> {
    let mut v: Vec<CellView> = Vec::new();
    thumb(&mut v, cols, rows, &bar(scroll, total));
    let mut rows: Vec<u16> = v
        .iter()
        .filter(|c| c.col == cols - 1)
        .map(|c| c.row)
        .collect();
    rows.sort_unstable();
    rows
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
    assert!(thumb_rows(40, MIN_ROWS - 1, 100, 5_000).is_empty());
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
    assert_eq!(count(&mut v, 37, 0, 0.5), 37, "no scroll, no change");
    assert!(v.is_empty());
    let next = count(&mut v, 37, 12, 0.5);
    assert_eq!(v.len(), 3, "⇡12 is three glyphs");
    assert!(next < 35, "and the next glyph goes to its left");
}

/// A card too narrow for the label leaves the border alone rather than
/// writing over the legend.
#[test]
fn a_count_with_no_room_writes_nothing() {
    let mut v: Vec<CellView> = Vec::new();
    assert_eq!(count(&mut v, 1, 4_000, 0.5), 1);
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
    let mut thumb_cells = Vec::new();
    thumb(&mut thumb_cells, 40, rows, &b);
    let thumb_fg = thumb_cells.first().expect("a thumb should be drawn").fg;
    let mut count_cells = Vec::new();
    count(
        &mut count_cells,
        30,
        b.scroll,
        position(total, visible, b.scroll),
    );
    let count_fg = count_cells.first().expect("a count should be drawn").fg;
    assert_eq!(thumb_fg, count_fg);
}
