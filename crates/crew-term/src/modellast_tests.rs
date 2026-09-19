use crate::{GridSize, HeadlessTerm, TermModel};

fn term(rows: u16, feed: &str) -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows });
    t.feed(feed.replace('\n', "\r\n").as_bytes());
    t
}

#[test]
fn an_empty_screen_has_no_last_line() {
    assert_eq!(term(4, "").last_line(), None);
}

/// The bottom-most row with text on it — not the last row of the screen, and
/// not the first line fed.
#[test]
fn the_bottom_most_written_row_wins() {
    let t = term(6, "building\nrunning tests\n");
    assert_eq!(t.last_line().as_deref(), Some("running tests"));
}

/// A program that leaves the caret mid-line (a shell prompt, a spinner) is
/// still showing that line — it does not need a newline to count.
#[test]
fn an_unterminated_row_counts() {
    let t = term(4, "~/code/crew $ ");
    assert_eq!(t.last_line().as_deref(), Some("~/code/crew $"));
}

/// Trailing blanks are padding, not content: alacritty fills every row to the
/// width of the grid.
#[test]
fn the_row_is_trimmed_at_the_end_only() {
    let t = term(4, "  indented line");
    assert_eq!(t.last_line().as_deref(), Some("  indented line"));
}

/// A wide character is one column of text, not two: the spacer cell parked
/// behind it must not become a space in the middle of the line.
#[test]
fn a_wide_character_does_not_bring_its_spacer() {
    let t = term(4, "\u{6e2c}\u{8a66} ok");
    assert_eq!(t.last_line().as_deref(), Some("\u{6e2c}\u{8a66} ok"));
}

/// Scrolled back, what the pane SHOWS is what it says — the rows above the
/// viewport are history, and the correction is the one `cells` makes.
#[test]
fn scrolled_back_the_visible_tail_is_the_line() {
    let mut t = term(3, "one\ntwo\nthree\nfour\n");
    assert_eq!(t.last_line().as_deref(), Some("four"));
    // Two lines up, the bottom of what is SHOWN is `three` — the caret's own
    // blank row and `four` are below the viewport now.
    t.scroll(2);
    assert_eq!(t.last_line().as_deref(), Some("three"));
}
