use crate::{GridSize, HeadlessTerm, TermModel};

fn term(bytes: &[u8]) -> HeadlessTerm {
    let mut t = HeadlessTerm::new(GridSize { cols: 10, rows: 3 });
    t.feed(bytes);
    t
}

/// vim's insert mode sends `CSI 5 SP q` (a blinking bar) and its normal mode
/// `CSI 2 SP q` (a steady block): the blink is the half of the request crew
/// used to throw away.
#[test]
fn decscusr_odd_shapes_blink_and_even_shapes_stand_still() {
    for (seq, blinks) in [
        (&b"\x1b[1 q"[..], true),
        (b"\x1b[2 q", false),
        (b"\x1b[3 q", true),
        (b"\x1b[4 q", false),
        (b"\x1b[5 q", true),
        (b"\x1b[6 q", false),
    ] {
        let t = term(seq);
        assert_eq!(
            t.cursor_blinks(),
            blinks,
            "{}",
            String::from_utf8_lossy(seq)
        );
    }
}

/// A cursor nobody styled is steady — and so is `CSI 0 SP q`, "back to the default".
#[test]
fn the_default_cursor_and_decscusr_0_are_steady() {
    assert!(!term(b"").cursor_blinks());
    assert!(!term(b"\x1b[5 q\x1b[0 q").cursor_blinks());
}

/// DECSET 12 turns the blink on and off without touching the shape.
#[test]
fn decset_12_toggles_the_blink_of_the_current_shape() {
    assert!(!term(b"\x1b[?12l").cursor_blinks());
    assert!(term(b"\x1b[6 q\x1b[?12h").cursor_blinks());
    assert!(!term(b"\x1b[5 q\x1b[?12l").cursor_blinks());
}

fn has_cursor(t: &HeadlessTerm, focused: bool) -> bool {
    t.cells(focused)
        .iter()
        .any(|c| c.cursor.shape != crew_theme::deco::CursorShape::None)
}

/// The dark half of a blink takes the cursor off the focused pane's cells —
/// and only the focused pane's: the outline an unfocused pane draws stays.
#[test]
fn a_dark_cursor_is_not_drawn_in_the_focused_pane_only() {
    let mut t = term(b"ab");
    assert!(has_cursor(&t, true));
    t.set_cursor_lit(false);
    assert!(!has_cursor(&t, true), "dark half draws no cursor");
    assert!(has_cursor(&t, false), "an unfocused outline never blinks");
    t.set_cursor_lit(true);
    assert!(has_cursor(&t, true));
}
