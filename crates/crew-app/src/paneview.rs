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
const BROADCAST: (u8, u8, u8) = (220, 120, 200);

/// Inputs for one pane's title bar.
struct Bar<'a> {
    cols: u16,
    index: Option<usize>,
    title: &'a str,
    focused: bool,
    /// Lines scrolled back from the live bottom (0 = at the bottom).
    scroll: usize,
    activity: bool,
    bell: bool,
    /// This pane is receiving broadcast (synchronized) input.
    broadcast: bool,
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
    // A magenta » marks panes receiving broadcast (synchronized) input.
    if b.broadcast {
        set(&mut out, x, '»', BROADCAST, bg);
        x += 2;
    }
    if let Some(n) = b.index {
        if (1..=9).contains(&n) {
            let d = char::from_digit(n as u32, 10).unwrap_or('?');
            set(&mut out, x, d, ACCENT, bg);
            x += 2;
        }
    }

    // Right-aligned status, stepping leftward: the scroll indicator `⇡N` (showing
    // how many lines back you are), then the activity and bell dots.
    let mut rx = b.cols.saturating_sub(2);
    if b.scroll > 0 {
        let s = format!("⇡{}", b.scroll);
        let w = s.chars().count() as u16;
        if rx + 1 > x + w {
            let start = rx + 1 - w;
            for (i, ch) in s.chars().enumerate() {
                set(&mut out, start + i as u16, ch, SCROLL_HINT, bg);
            }
            rx = start.saturating_sub(2);
        }
    }
    for (on, c, fg) in [(b.activity, '●', ACTIVITY), (b.bell, '!', BELL)] {
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
/// `broadcast` marks terminal panes that receive synchronized input; `find` is
/// the active `/find` term, highlighted in the focused pane while scrolled back.
pub fn build_scenes(
    panes: &[Pane],
    focused: Option<usize>,
    broadcast: bool,
    find: Option<&str>,
) -> Vec<PaneScene> {
    let multi = panes.len() > 1;
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let foc = focused == Some(i);
            let mut cells = p.cells(foc);
            let is_term = matches!(&p.content, PaneContent::Terminal(_));
            let scroll = match &p.content {
                PaneContent::Terminal(t) => t.pty.display_offset(),
                _ => 0,
            };
            // Tint http(s) URLs blue so they read as clickable (Cmd+click opens).
            if is_term {
                crate::linkhl::colorize(&mut cells, p.grid.cols, p.grid.rows);
            }
            // Wash search matches in the focused terminal while viewing a /find
            // result (scrolled back); it self-clears on return to the bottom.
            if foc && is_term && scroll > 0 {
                if let Some(term) = find {
                    crate::findhl::highlight(&mut cells, term, p.grid.cols, p.grid.rows);
                }
            }
            for c in cells.iter_mut() {
                c.row += 1; // content sits below the title bar
            }
            let title = p.title_text();
            cells.extend(title_bar(&Bar {
                cols: p.grid.cols,
                index: multi.then_some(i + 1),
                title: &title,
                focused: foc,
                scroll,
                activity: p.activity && !foc,
                bell: p.bell && !foc,
                broadcast: broadcast && is_term,
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
#[path = "paneview_tests.rs"]
mod tests;
