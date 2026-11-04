use crate::{GridSize, HeadlessTerm, TermModel};

fn cells(bytes: &[u8]) -> Vec<crate::RenderCell> {
    let mut t = HeadlessTerm::new(GridSize { cols: 20, rows: 2 });
    t.feed(bytes);
    t.cells(false)
}

/// vim's status line, nano's bars, less's `(END)`: on the alternate screen a
/// reversed cell swaps its colours, and the reversed blanks that make the bar
/// are drawn, not dropped as empty cells.
#[test]
fn reverse_video_draws_a_bar_on_the_alternate_screen() {
    let cells = cells(b"\x1b[?1049hX\x1b[7mH  \x1b[0m");
    let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
    let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
    assert_eq!(h.bg, x.fg, "the reversed cell's bg is the normal fg");
    assert_ne!(h.bg, x.bg, "the reversed cell shows a bar");
    let bar: Vec<_> = cells
        .iter()
        .filter(|c| c.row == 0 && (2..=3).contains(&c.col))
        .collect();
    assert_eq!(bar.len(), 2, "both reversed spaces are drawn: {bar:?}");
    assert!(bar.iter().all(|c| c.bg == h.bg), "the spaces carry the bar");
}

/// The swapped background is the default foreground: light grey on a dark
/// theme, which the echo-grey drop would flatten. It must survive.
#[test]
fn the_reversed_bar_is_not_flattened_by_the_echo_grey_drop() {
    let cells = cells(b"\x1b[?1049h\x1b[7m \x1b[0m");
    let bar = cells.iter().find(|c| c.c == ' ').expect("bar cell drawn");
    assert_eq!(bar.bg, crate::color::default_fg());
}

/// Leaving the alternate screen restores the primary-screen rule: the line an
/// agent CLI echoes in reverse video stays plain.
#[test]
fn the_primary_screen_still_ignores_reverse_video() {
    let cells = cells(b"\x1b[?1049h\x1b[?1049lX\x1b[7mH\x1b[0m");
    let x = cells.iter().find(|c| c.c == 'X').expect("X rendered");
    let h = cells.iter().find(|c| c.c == 'H').expect("H rendered");
    assert_eq!((h.fg, h.bg), (x.fg, x.bg));
}
