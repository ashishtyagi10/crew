//! A picture keeps moving with its text after the scrollback is full, and
//! goes when that text is thrown away. A 20-line scrollback fills fast.
use crate::model::{GridSize, HeadlessTerm, TermModel};
use crate::wipescan::ImageWipe;

/// An 80×40 picture claiming two rows of a 10×20 cell.
fn picture() -> String {
    use base64::Engine;
    let mut png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    png.extend(80u32.to_be_bytes());
    png.extend(40u32.to_be_bytes());
    png.extend([8, 6, 0, 0, 0]);
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    format!("\x1b_Ga=T,f=100;{b64}\x1b\\")
}

/// A 12-row pane with a 20-line scrollback, already full.
fn full() -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 40, rows: 12 });
    t.set_cell_px(10, 20);
    t.set_scrollback(20);
    t.feed("x\r\n".repeat(100).as_bytes());
    assert_eq!(t.history_lines(), 20, "full");
    t
}

/// The row of the screen a line is on now.
fn row(t: &HeadlessTerm, line: u64) -> i64 {
    line as i64 - t.screen_line() as i64
}

#[test]
fn a_picture_rises_with_its_text_after_the_scrollback_is_full() {
    let mut t = full();
    t.feed(picture().as_bytes());
    let img = t.take_images().remove(0);
    let before = row(&t, img.line);
    t.feed("y\r\n".repeat(5).as_bytes());
    assert_eq!(t.history_lines(), 20, "the history no longer grows");
    assert_eq!(row(&t, img.line), before - 5, "the picture still climbs");
}

/// Every line counts, however it scrolls: one big read of line feeds, a
/// scroll-up, and a wrap past the last column.
#[test]
fn every_scrolled_line_is_counted_in_one_big_read() {
    let mut t = full();
    let top = t.screen_line();
    t.feed("\n".repeat(5000).as_bytes());
    assert_eq!(t.screen_line(), top + 5000);
    t.feed(b"\x1b[7S");
    assert_eq!(t.screen_line(), top + 5007, "CSI S");
    t.feed("w".repeat(40 * 3).as_bytes());
    assert_eq!(t.screen_line(), top + 5009, "two wraps past the last row");
}

#[test]
fn a_picture_whose_text_was_thrown_away_is_dropped() {
    let mut t = full();
    t.feed(picture().as_bytes());
    let img = t.take_images().remove(0);
    t.take_image_wipes();
    // Up through the screen and the whole scrollback, and out of the top.
    t.feed("y\r\n".repeat(40).as_bytes());
    let wipes = t.take_image_wipes();
    assert_eq!(wipes.len(), 1, "one wipe for the read: {wipes:?}");
    assert!(!wipes[0].keeps(img.line, false), "{wipes:?}");
    assert!(wipes[0].keeps(t.screen_line(), false), "the screen's stay");
    // One not yet handed over goes in the core.
    t.feed(picture().as_bytes());
    t.feed("y\r\n".repeat(40).as_bytes());
    assert!(t.take_images().is_empty());
}

/// `clear` drops what is on the screen NOW. On a full history the old count
/// had frozen, so a picture already scrolled up into the scrollback still
/// looked to be on the screen and went with it.
#[test]
fn clear_on_a_full_scrollback_keeps_the_pictures_above_the_screen() {
    let mut t = full();
    t.feed(picture().as_bytes());
    let img = t.take_images().remove(0);
    t.feed("y\r\n".repeat(15).as_bytes());
    assert!(row(&t, img.line) < 0, "scrolled up into the history");
    t.take_image_wipes();
    let top = t.screen_line();
    t.feed(b"\x1b[H\x1b[2J");
    // The erase itself pushes the screen into the history, evicting more.
    let wipes = t.take_image_wipes();
    assert_eq!(
        wipes[0],
        ImageWipe::Screen {
            line: top,
            alt: false
        }
    );
    assert!(wipes.iter().all(|w| w.keeps(img.line, false)), "{wipes:?}");
}

/// The spare room the counting borrows is the primary's only: the
/// alternate screen never starts keeping history.
#[test]
fn the_alternate_screen_keeps_no_history() {
    let mut t = full();
    // In one read: the pieces after the switch are on the alternate screen.
    t.feed(format!("\x1b[?1049h{}", "a\n".repeat(500)).as_bytes());
    assert_eq!(t.history_lines(), 0);
    t.feed(b"\x1b[?1049l");
    assert_eq!(t.history_lines(), 20);
    assert!(!t.cells(false).is_empty());
}
