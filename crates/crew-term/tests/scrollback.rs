//! Scrollback + feed behaviour of the headless terminal model.
use crew_term::{GridSize, HeadlessTerm, TermModel};

#[test]
fn sgr_red_bold_is_resolved_to_rgb_and_flags() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    term.feed(b"\x1b[1m\x1b[31mX"); // bold + red foreground
    let cell = term
        .cells(true)
        .into_iter()
        .find(|c| c.c == 'X')
        .expect("cell X");
    assert!(cell.bold);
    // Red resolves from the active theme's ANSI palette slot 1; the theme's red
    // has a dominant red channel, so check R > G and R > B with a clear margin.
    let expected = crew_theme::theme().ansi[1];
    assert_eq!(
        cell.fg, expected,
        "fg should be the theme's red ({expected:?}), got {:?}",
        cell.fg
    );
}

#[test]
fn cursor_block_rendered_at_live_position() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    term.feed(b"hi");
    // The cursor sits after "hi" at column 2. This used to assert the two
    // literal RGB constants the block was drawn in — which is how they went
    // eight releases without anyone noticing that on a light theme the
    // FOCUSED cursor read at 1.5 against the page and the unfocused one at
    // 6.2. It asserts the contract instead: a block is drawn, and the focused
    // one out-reads the unfocused one against the page it sits on.
    let block = |focused: bool| {
        term.cells(focused)
            .into_iter()
            .find(|c| c.col == 2 && c.row == 0)
            .map(|c| c.bg)
    };
    let (on, off) = (block(true).expect("focused"), block(false).expect("un"));
    assert_ne!(on, off, "focus must change the cursor");
    let page = crew_theme::theme().term_bg;
    assert!(
        crew_theme::contrast_ratio(on, page) > crew_theme::contrast_ratio(off, page),
        "the focused cursor must be the one that reads"
    );
}

#[test]
fn osc_title_is_captured() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    assert_eq!(term.title(), "");
    term.feed(b"\x1b]2;~/code/crew\x07"); // OSC 2 set-window-title
    assert_eq!(term.title(), "~/code/crew");
}

#[test]
fn osc7_cwd_report_is_captured() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    assert_eq!(term.take_cwd(), None);
    // A shell reports its directory via OSC 7 on each prompt — the ANSI parser
    // ignores it, so the model sniffs it out of the stream.
    term.feed(b"\x1b]7;file://host/Users/me/code/crew\x07");
    assert_eq!(
        term.take_cwd(),
        Some(std::path::PathBuf::from("/Users/me/code/crew"))
    );
    // One-shot: nothing to take until it changes again.
    assert_eq!(term.take_cwd(), None);
}

#[test]
fn osc52_clipboard_store_is_captured() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    assert_eq!(term.take_clipboard(), None);
    term.feed(b"\x1b]52;c;aGVsbG8=\x07"); // OSC 52 set clipboard to base64("hello")
    assert_eq!(term.take_clipboard().as_deref(), Some("hello"));
    assert_eq!(term.take_clipboard(), None, "taking clears it");
}

#[test]
fn bell_is_captured_and_cleared() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    assert!(!term.take_bell());
    term.feed(b"\x07"); // BEL
    assert!(term.take_bell());
    assert!(!term.take_bell(), "taking clears it");
}

#[test]
fn feeding_text_appears_in_cells() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 5 });
    term.feed(b"hi");
    // cols 0..2 are the text; col 2 holds the cursor block (a space).
    let mut row0: Vec<_> = term
        .cells(true)
        .into_iter()
        .filter(|c| c.row == 0 && c.col < 2)
        .collect();
    row0.sort_by_key(|c| c.col);
    let text: String = row0.iter().map(|c| c.c).collect();
    assert_eq!(text, "hi");
}

#[test]
fn scrolling_up_reveals_lines_pushed_into_history() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    term.feed(b"TOPLINE\r\n");
    for _ in 0..20 {
        term.feed(b"x\r\n");
    }
    let live: String = term.cells(true).iter().map(|c| c.c).collect();
    assert!(
        !live.contains('T'),
        "TOPLINE should be off-screen at the live bottom"
    );

    assert_eq!(term.display_offset(), 0, "starts at the live bottom");
    term.scroll(1000); // clamps to the top of the scrollback
    assert!(term.display_offset() > 0, "scrolled back from the bottom");
    let scrolled: String = term.cells(true).iter().map(|c| c.c).collect();
    assert!(
        scrolled.contains('T') && scrolled.contains('P'),
        "TOPLINE should reappear after scrolling to the top, got {scrolled:?}"
    );
}

#[test]
fn clear_scrollback_drops_history() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    term.feed(b"TOPLINE\r\n");
    for _ in 0..20 {
        term.feed(b"x\r\n");
    }
    term.scroll(1000);
    assert!(term.display_offset() > 0, "has scrollback before clearing");

    term.feed(b"\x1b[3J"); // CSI 3 J: clear saved (scrollback) lines

    // With the history gone, scrolling up can no longer leave the live bottom.
    term.scroll(1000);
    assert_eq!(
        term.display_offset(),
        0,
        "no scrollback should remain after clearing"
    );
}
