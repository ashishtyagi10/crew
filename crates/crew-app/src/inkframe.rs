//! The frame you are in, on an Ink Reader page: drawn in heavy rules.
//!
//! Print has no glow to say which pane has focus, so the 2026-10-16 pages
//! said it in tone alone: a black line among grey ones. The user, on
//! 2026-10-17: "make the focused frame thicker too". So the focused card
//! swaps its light rules for the heavy ones of the same box set (`━ ┃ ┏ ┓
//! ┗ ┛`), which the renderer draws at twice the light weight and rounds at
//! card scale as it does the light ones.
//!
//! This runs last, on the finished cells. Everything before it (the
//! assembly, the legend's floor, the brackets, the tints) finds the frame by
//! its light rules.
use crew_render::CellView;

/// The heavy rule standing in for the light one `c`, or `c` itself.
fn heavy(c: char) -> char {
    match c {
        '\u{2500}' => '\u{2501}',
        '\u{2502}' => '\u{2503}',
        '\u{256D}' => '\u{250F}',
        '\u{256E}' => '\u{2513}',
        '\u{256F}' => '\u{251B}',
        '\u{2570}' => '\u{2517}',
        c => c,
    }
}

/// A `cols`×`rows` card's frame in heavy rules when it has focus on a print
/// page. Glass and the tubes keep their light line, lit by the ring or the
/// trace instead.
pub(crate) fn weigh(cells: &mut [CellView], (cols, rows): (u16, u16), focused: bool) {
    if !focused || !crew_theme::theme().is_print() {
        return;
    }
    let edge =
        |c: &&mut CellView| c.row == 0 || c.row + 1 == rows || c.col == 0 || c.col + 1 == cols;
    for c in cells.iter_mut().filter(edge) {
        c.c = heavy(c.c);
    }
}

/// For a test that names the light frame's glyphs: the theme lock, on a glass
/// theme, where a focused frame is the light rule.
#[cfg(test)]
pub(crate) fn glass_theme_test_guard() -> crate::appearanceguard::ThemeGuard {
    let g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Nebula);
    g
}
