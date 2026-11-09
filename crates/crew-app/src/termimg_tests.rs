//! Where a picture is, once the text it arrived in has moved.
use super::*;

/// A store holding one already-decoded picture at `line`, `cells` big.
fn at(line: u64, col: u16, cells: (u16, u16)) -> TermImages {
    TermImages {
        shown: vec![Shown {
            line,
            alt: false,
            id: 0,
            col,
            cells,
            art: Art::Ready(Bitmap {
                w: 2,
                h: 2,
                px: vec![[200, 30, 30, 255]; 4],
                src: (2, 2),
            }),
        }],
        alt: false,
    }
}

/// The rows a store's paint covers, or `None` when it drew nothing.
fn rows_of(t: &TermImages, history: u64, offset: usize, rows: u16) -> Option<(f32, f32)> {
    let ps = t.paint(history, offset, 40, rows, 2.0);
    let first = ps.first()?;
    Some(ps.iter().fold((first.y, first.y + first.h), |a, p| {
        (a.0.min(p.y), a.1.max(p.y + p.h))
    }))
}

/// The anchor is an absolute buffer line, so the picture climbs the screen as
/// output pushes it up — the whole reason it is not stored as a screen row.
#[test]
fn a_picture_rises_with_the_text_it_arrived_in() {
    let t = at(10, 0, (8, 4));
    let fresh = rows_of(&t, 10, 0, 24).expect("drawn where it landed");
    assert!(fresh.0 >= -0.01 && fresh.0 < 1.0, "starts at the top row");
    // Twelve more lines of output: it is twelve rows further up, and gone.
    let later = rows_of(&t, 22, 0, 24);
    assert!(later.is_none() || later.expect("some").0 < fresh.0);
    assert!(
        rows_of(&t, 40, 0, 24).is_none(),
        "a picture far above the window is not rasterized at all"
    );
}

/// …and scrolling back must bring it into view again, at the row it belongs
/// to, or a picture would be a thing you could only ever see once.
#[test]
fn scrolling_back_finds_it_again() {
    let t = at(10, 0, (8, 4));
    assert!(rows_of(&t, 40, 0, 24).is_none(), "off the top");
    let back = rows_of(&t, 40, 34, 24).expect("scrolled back to it");
    assert!(
        back.0 >= -0.01 && back.0 < 5.0,
        "back on screen at {back:?}"
    );
}

/// Paint is free rectangles — nothing else clips it — so a picture half off
/// the top of the pane must be cut, not drawn over the pane above.
#[test]
fn a_picture_hanging_off_the_top_is_clipped_to_the_pane() {
    let t = at(10, 0, (8, 6));
    let (y0, y1) = rows_of(&t, 13, 0, 24).expect("partly visible");
    assert!(y0 >= -0.01, "drew above the pane's first row: {y0}");
    assert!(y1 <= 24.01, "drew below the pane's last row: {y1}");
}

#[test]
fn a_picture_below_the_window_is_not_drawn() {
    let t = at(100, 0, (8, 4));
    assert!(rows_of(&t, 10, 0, 24).is_none());
}

/// A pane that draws a chart a second must not accumulate charts forever.
#[test]
fn only_the_last_few_pictures_are_kept() {
    let mut t = TermImages::default();
    for i in 0..(KEEP + 10) {
        t.collect(vec![crew_term::PlacedImage {
            line: i as u64,
            col: 0,
            cells: (4, 2),
            alt: false,
            cmd: crew_term::ImageCmd::default(),
        }]);
    }
    assert_eq!(t.shown.len(), KEEP);
    assert_eq!(t.shown[0].line, 10, "the oldest went first");
}

/// Decoding is the worker's job; a payload that is not a picture must come
/// back as a failure rather than as a panic or a hang.
#[test]
fn a_payload_that_is_not_a_picture_lands_as_a_failure() {
    let mut t = TermImages::default();
    t.collect(vec![crew_term::PlacedImage {
        line: 0,
        col: 0,
        cells: (4, 2),
        alt: false,
        cmd: crew_term::ImageCmd {
            action: b'T',
            format: 100,
            data: b"nonsense".to_vec(),
            ..Default::default()
        },
    }]);
    for _ in 0..200 {
        if t.poll() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(!t.loading(), "the worker must always report back");
    assert!(t.paint(0, 0, 40, 24, 2.0).is_empty(), "and draw nothing");
}

/// What a pane shows after `kitten icat x.png; clear`, driven through a real
/// terminal core: a young pane's history is emptied by `clear`, so without
/// the wipe the picture's anchor (line 1) is back on screen, over the prompt.
#[test]
fn clear_takes_the_picture_with_it() {
    let mut term = crew_term::HeadlessTerm::new(crew_term::GridSize { cols: 40, rows: 12 });
    use crew_term::TermModel;
    term.feed(b"$ icat x.png\r\n");
    let mut t = at(1, 0, (8, 2));
    assert!(
        rows_of(&t, 0, 0, 12).is_some(),
        "on screen before the clear"
    );
    term.feed(b"\x1b[H\x1b[2J\x1b[3J$ ");
    t.sync(term.take_image_wipes(), term.take_images());
    assert_eq!(term.history_lines(), 0, "the clear emptied the history");
    assert!(
        rows_of(&t, 0, 0, 12).is_none(),
        "a ghost over the new prompt"
    );
}

/// A full-screen program's preview is placed on the alternate screen: it is
/// drawn only there, and it is gone once the program quits; the shell's own
/// pictures hide under the program and come back after it.
#[test]
fn the_alternate_screen_keeps_its_pictures_to_itself() {
    let mut term = crew_term::HeadlessTerm::new(crew_term::GridSize { cols: 40, rows: 12 });
    use crew_term::TermModel;
    let mut t = at(2, 0, (8, 2));
    term.feed(b"\x1b[?1049h");
    t.sync(term.take_image_wipes(), Vec::new());
    assert!(
        rows_of(&t, 0, 0, 12).is_none(),
        "the shell's picture over vim"
    );
    let mut preview = at(4, 20, (8, 2)).shown.remove(0);
    preview.alt = true;
    t.shown.push(preview);
    assert!(
        rows_of(&t, 0, 0, 12).is_some(),
        "the preview is drawn in the program"
    );
    term.feed(b"\x1b[?1049l");
    t.sync(term.take_image_wipes(), Vec::new());
    assert_eq!(t.shown.len(), 1, "the preview left with the program");
    assert!(!t.shown[0].alt, "and the shell's picture stayed");
    assert!(
        rows_of(&t, 0, 0, 12).is_some(),
        "drawn again on the shell's screen"
    );
}
