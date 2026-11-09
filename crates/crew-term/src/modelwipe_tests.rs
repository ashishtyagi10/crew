//! Pictures leave with the text they arrived in: `clear`, the alternate
//! screen and kitty's delete, through a whole terminal core.
use crate::model::{GridSize, HeadlessTerm, TermModel};
use crate::wipescan::ImageWipe;

fn picture() -> String {
    use base64::Engine;
    let mut png = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    png.extend(80u32.to_be_bytes());
    png.extend(40u32.to_be_bytes());
    png.extend([8, 6, 0, 0, 0]);
    let b64 = base64::engine::general_purpose::STANDARD.encode(png);
    format!("\x1b_Ga=T,f=100;{b64}\x1b\\")
}

fn term() -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 40, rows: 12 });
    t.set_cell_px(10, 20);
    t
}

/// Whether a picture taken earlier survives the wipes taken now.
fn survives(t: &mut HeadlessTerm, line: u64, alt: bool) -> bool {
    t.take_image_wipes().iter().all(|w| w.keeps(line, alt, 0))
}

#[test]
fn clear_drops_the_picture_a_young_pane_would_draw_over_the_prompt() {
    let mut t = term();
    t.feed(format!("$ icat x.png\r\n{}$ ", picture()).as_bytes());
    let img = t.take_images().remove(0);
    assert_eq!((img.line, img.alt), (1, false));
    t.feed(b"clear\r\n\x1b[H\x1b[2J\x1b[3J$ ");
    assert_eq!(t.history_lines(), 0, "the anchor is back on screen");
    assert!(!survives(&mut t, img.line, img.alt));
}

#[test]
fn a_picture_placed_and_cleared_in_one_read_is_never_handed_over() {
    let mut t = term();
    t.feed(format!("{}\x1b[H\x1b[2J\x1b[3Jafter", picture()).as_bytes());
    assert_eq!(t.take_images(), []);
}

/// `ESC[2J` alone moves the screen into the scrollback: the pictures on the
/// screen go, the ones already scrolled into history stay with their text.
#[test]
fn erasing_the_screen_keeps_the_scrollback_pictures() {
    let mut t = term();
    t.feed(format!("{}{}", picture(), "x\r\n".repeat(20)).as_bytes());
    let old = t.take_images().remove(0);
    t.feed(picture().as_bytes());
    let new = t.take_images().remove(0);
    t.feed(b"\x1b[2J");
    let wipes = t.take_image_wipes();
    assert!(
        wipes.iter().all(|w| w.keeps(old.line, false, 0)),
        "{wipes:?}"
    );
    assert!(
        !wipes.iter().all(|w| w.keeps(new.line, false, 0)),
        "{wipes:?}"
    );
}

#[test]
fn the_alternate_screen_takes_its_pictures_when_it_goes() {
    let mut t = term();
    t.feed(format!("$ ls\r\n{}$ yazi\r\n", picture()).as_bytes());
    let shell = t.take_images().remove(0);
    t.feed(format!("\x1b[?1049h\x1b[5;20H{}", picture()).as_bytes());
    let preview = t.take_images().remove(0);
    assert!(preview.alt && !shell.alt);
    assert_eq!(t.take_image_wipes(), [ImageWipe::Alt(true)]);
    t.feed(b"\x1b[?1049l$ ");
    let wipes = t.take_image_wipes();
    assert_eq!(wipes, [ImageWipe::Alt(false)]);
    assert!(
        !wipes[0].keeps(preview.line, preview.alt, 0),
        "no ghost preview"
    );
    assert!(
        wipes[0].keeps(shell.line, shell.alt, 0),
        "the shell's own stays"
    );
}

#[test]
fn a_kitty_delete_reaches_the_pictures_already_handed_over() {
    let mut t = term();
    t.feed(picture().as_bytes());
    let img = t.take_images().remove(0);
    t.feed(b"\x1b_Ga=d\x1b\\");
    assert!(!survives(&mut t, img.line, img.alt));
}

#[test]
fn ordinary_output_drops_nothing() {
    let mut t = term();
    t.feed(format!("{}\x1b[K\x1b[J\x1b[?1006h{}", picture(), "y\r\n".repeat(30)).as_bytes());
    assert_eq!(t.take_image_wipes(), []);
    assert_eq!(t.take_images().len(), 1);
}

/// `picture()` sent as kitty id `id`, quietly (`q=2`): no replies to drain.
fn picture_id(id: u32) -> String {
    picture().replacen("a=T,", &format!("a=T,i={id},q=2,"), 1)
}

/// A previewer deleting its last preview by id takes that one alone — from
/// the pictures not yet handed over and the ones the app already holds.
#[test]
fn a_kitty_delete_by_id_takes_only_that_picture() {
    let mut t = term();
    t.feed(format!("{}{}\x1b_Ga=d,d=i,i=1\x1b\\", picture_id(1), picture_id(2)).as_bytes());
    let held: Vec<u32> = t.take_images().iter().map(|p| p.cmd.id).collect();
    assert_eq!(held, [2]);
    t.feed(picture_id(3).as_bytes());
    let img = t.take_images().remove(0);
    t.feed(b"\x1b_Ga=d,d=I,i=1\x1b\\");
    let wipes = t.take_image_wipes();
    assert!(
        wipes.iter().all(|w| w.keeps(img.line, img.alt, 3)),
        "{wipes:?}"
    );
    assert!(
        !wipes.iter().all(|w| w.keeps(img.line, img.alt, 1)),
        "{wipes:?}"
    );
    assert!(
        wipes.iter().all(|w| w.keeps(img.line, img.alt, 0)),
        "no id, not named"
    );
    // A delete by id that names none takes none — not the pictures sent
    // without an id.
    t.feed(b"\x1b_Ga=d,d=i\x1b\\");
    let wipes = t.take_image_wipes();
    assert!(
        wipes.iter().all(|w| w.keeps(img.line, img.alt, 0)),
        "{wipes:?}"
    );
}
