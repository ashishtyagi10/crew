//! Rendering panes to `PaneScene`s. Each pane reserves its top row for a title
//! bar carrying the index, the program-set title, and status glyphs (scrollback,
//! activity, bell); the pane content is shifted down one row beneath it.
use crew_render::{CellView, PaneScene};

use crate::pane::{Pane, PaneContent};

const ACCENT: (u8, u8, u8) = (0, 255, 160);
const SCROLL_HINT: (u8, u8, u8) = (230, 180, 90);
const ACTIVITY: (u8, u8, u8) = (120, 200, 255);
const BELL: (u8, u8, u8) = (240, 210, 90);
const BAR_FOCUSED: (u8, u8, u8) = (28, 40, 46);
const BAR_DIM: (u8, u8, u8) = (20, 20, 28);
const TITLE_ON: (u8, u8, u8) = (225, 225, 225);
const TITLE_OFF: (u8, u8, u8) = (140, 140, 150);

/// Inputs for one pane's title bar.
struct Bar<'a> {
    cols: u16,
    index: Option<usize>,
    title: &'a str,
    focused: bool,
    scrolled: bool,
    activity: bool,
    bell: bool,
}

fn cell(col: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> CellView {
    CellView {
        col,
        row: 0,
        c,
        fg,
        bg,
        bold: false,
        italic: false,
    }
}

fn set(out: &mut [CellView], col: u16, c: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) {
    if (col as usize) < out.len() {
        out[col as usize] = cell(col, c, fg, bg);
    }
}

/// Build the title-bar cells for row 0: a filled bar with the index, title, and
/// right-aligned status glyphs.
fn title_bar(b: &Bar) -> Vec<CellView> {
    if b.cols < 4 {
        return Vec::new();
    }
    let bg = if b.focused { BAR_FOCUSED } else { BAR_DIM };
    let mut out: Vec<CellView> = (0..b.cols).map(|c| cell(c, ' ', bg, bg)).collect();

    let mut x = 1u16;
    if let Some(n) = b.index {
        if (1..=9).contains(&n) {
            let d = char::from_digit(n as u32, 10).unwrap_or('?');
            set(&mut out, x, d, ACCENT, bg);
            x += 2;
        }
    }

    // Right-aligned status glyphs, stepping leftward.
    let mut rx = b.cols.saturating_sub(2);
    for (on, c, fg) in [
        (b.bell, '!', BELL),
        (b.activity, '●', ACTIVITY),
        (b.scrolled, '⇡', SCROLL_HINT),
    ] {
        if on && rx > x {
            set(&mut out, rx, c, fg, bg);
            rx = rx.saturating_sub(2);
        }
    }

    let fg = if b.focused { TITLE_ON } else { TITLE_OFF };
    for (i, ch) in b.title.chars().enumerate() {
        let col = x + i as u16;
        if col >= rx {
            break;
        }
        set(&mut out, col, ch, fg, bg);
    }
    out
}

/// Build a `Vec<PaneScene>` from the current pane state (for `renderer.frame`).
pub fn build_scenes(panes: &[Pane], focused: Option<usize>) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let foc = focused == Some(i);
            let mut cells = p.cells(foc);
            for c in cells.iter_mut() {
                c.row += 1; // content sits below the title bar
            }
            let scrolled =
                matches!(&p.content, PaneContent::Terminal(t) if t.pty.display_offset() > 0);
            let title = p.title_text();
            cells.extend(title_bar(&Bar {
                cols: p.grid.cols,
                index: multi.then_some(i + 1),
                title: &title,
                focused: foc,
                scrolled,
                activity: p.activity && !foc,
                bell: p.bell && !foc,
            }));
            PaneScene {
                cells,
                x: p.rect.x,
                y: p.rect.y,
                w: p.rect.w,
                h: p.rect.h,
                focused: foc,
                bordered: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(focused: bool) -> Bar<'static> {
        Bar {
            cols: 40,
            index: Some(2),
            title: "~/code",
            focused,
            scrolled: true,
            activity: true,
            bell: true,
        }
    }

    #[test]
    fn title_bar_has_index_title_and_glyphs() {
        let cells = title_bar(&bar(true));
        assert_eq!(cells.len(), 40); // full-width bar
        assert!(cells.iter().any(|c| c.c == '2' && c.fg == ACCENT));
        assert!(cells.iter().any(|c| c.c == '~'));
        assert!(cells.iter().any(|c| c.c == '⇡' && c.fg == SCROLL_HINT));
        assert!(cells.iter().any(|c| c.c == '●' && c.fg == ACTIVITY));
        assert!(cells.iter().any(|c| c.c == '!' && c.fg == BELL));
        assert!(cells.iter().all(|c| c.row == 0));
    }

    #[test]
    fn title_bar_bg_differs_by_focus() {
        assert_ne!(title_bar(&bar(true))[0].bg, title_bar(&bar(false))[0].bg);
    }
}
