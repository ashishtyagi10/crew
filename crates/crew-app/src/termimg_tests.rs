//! Where a picture is, once the text it arrived in has moved.
use super::*;

/// A store holding one already-decoded picture at `line`, `cells` big.
fn at(line: u64, col: u16, cells: (u16, u16)) -> TermImages {
    TermImages {
        shown: vec![Shown {
            line,
            col,
            cells,
            art: Art::Ready(Bitmap {
                w: 2,
                h: 2,
                px: vec![[200, 30, 30, 255]; 4],
                src: (2, 2),
            }),
        }],
    }
}

/// The rows a store's paint covers, or `None` when it drew nothing.
fn rows_of(t: &TermImages, history: usize, offset: usize, rows: u16) -> Option<(f32, f32)> {
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
