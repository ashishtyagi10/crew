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
    assert!(
        cell.fg.0 > 120 && cell.fg.1 < 100 && cell.fg.2 < 100,
        "fg should be reddish, got {:?}",
        cell.fg
    );
}

#[test]
fn cursor_block_rendered_at_live_position() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    term.feed(b"hi");
    // the cursor sits after "hi" at column 2 — a bright block when focused,
    // a dim block when not.
    assert!(term
        .cells(true)
        .iter()
        .any(|c| c.col == 2 && c.row == 0 && c.bg == (200, 200, 200)));
    assert!(term
        .cells(false)
        .iter()
        .any(|c| c.col == 2 && c.row == 0 && c.bg == (90, 90, 100)));
}

#[test]
fn osc_title_is_captured() {
    let mut term = HeadlessTerm::new(GridSize { cols: 20, rows: 3 });
    assert_eq!(term.title(), "");
    term.feed(b"\x1b]2;~/code/farx\x07"); // OSC 2 set-window-title
    assert_eq!(term.title(), "~/code/farx");
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
