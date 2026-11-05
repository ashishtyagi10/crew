use ratatui::layout::Rect;

use super::*;
use crate::config::CrewConfig;
use crate::settingspane::click::{hit, Hit};
use crate::settingspane::{form, SettingsPane, FIELDS};
use crew_render::CellView;

fn at(cells: &[CellView], col: u16, row: u16) -> Option<char> {
    cells
        .iter()
        .find(|c| c.col == col && c.row == row)
        .map(|c| c.c)
}

/// The scroll the renderer picks, for a pane focused on `focus`.
fn off_of(p: &SettingsPane, lay: &form::FormLayout, viewport: u16) -> u16 {
    let tail = Rect::new(0, lay.height.saturating_sub(1), 1, 1);
    let r = lay.rect_of(p.focused_field()).unwrap_or(tail);
    form::scroll_for(r, lay.height, viewport)
}

/// At every pane height that cuts the form, and focused on each field in
/// turn (so it scrolls), no boxed field shows one end without the other. The
/// survey's `Min secs` legend over nothing and `Line spacing` box missing its
/// value row were both a `╭` with no `╰` under it.
#[test]
fn a_cut_form_never_shows_half_a_field() {
    let _g = crate::app::theme_test_guard();
    let mut checked = 0;
    for cols in [48u16, 80] {
        let lay = form::layout(cols);
        for rows in 8..lay.height + 2 {
            let viewport = rows - 2;
            for focus in 0..FIELDS.len() - 2 {
                let mut p = SettingsPane::new(CrewConfig::default(), Vec::new());
                p.focus = focus;
                let cells = p.cells(cols, rows);
                let off = off_of(&p, &lay, viewport);
                for &(f, r) in lay.rects.iter().filter(|(_, r)| r.height >= 2) {
                    if f == p.focused_field() {
                        continue; // a focused box taller than the pane is the one exception
                    }
                    let row = |y: u16| y.checked_sub(off).filter(|&v| v < viewport);
                    let (top, foot) = (row(r.y), row(r.y + r.height - 1));
                    let corner =
                        |y: Option<u16>, c| y.is_some_and(|y| at(&cells, r.x, y) == Some(c));
                    let (t, b) = (corner(top, '\u{256D}'), corner(foot, '\u{2570}'));
                    checked += usize::from(t || b);
                    assert_eq!(
                        t, b,
                        "{f:?} half drawn at {cols}x{rows} (focus {focus}): rows {top:?}..{foot:?}"
                    );
                }
            }
        }
    }
    assert!(checked > 100, "the sweep saw whole fields ({checked})");
}

/// A field left out at the cut answers no click: a field you can't see is
/// not a field you meant to press.
#[test]
fn a_field_left_out_at_the_cut_takes_no_click() {
    let _g = crate::app::theme_test_guard();
    let p = SettingsPane::new(CrewConfig::default(), Vec::new());
    let cols = 48;
    let lay = form::layout(cols);
    let mut seen = 0;
    for rows in 8..lay.height {
        let viewport = rows - 2;
        for &(f, r) in &lay.rects {
            // Across the bottom edge, at the top of the form (focus 0).
            if f == p.focused_field() || !(r.y < viewport && r.y + r.height > viewport) {
                continue;
            }
            seen += 1;
            for row in r.y..viewport {
                let got = hit(&p, cols, rows, row, r.x + 1);
                assert!(
                    !matches!(got, Some(Hit::Field(g) | Hit::Step(g, _)) if g == f),
                    "{f:?} answered a click at {rows} rows"
                );
            }
        }
    }
    assert!(seen > 0, "some height cuts a field");
}

#[test]
fn split_means_partly_in_view() {
    let r = Rect::new(0, 4, 10, 3); // rows 4..7
    assert!(split(r, 0, 6, false), "bottom edge through it");
    assert!(split(r, 5, 6, false), "top edge through it");
    assert!(!split(r, 0, 7, false), "whole");
    assert!(!split(r, 0, 4, false), "wholly below");
    assert!(!split(r, 7, 4, false), "wholly above");
    assert!(!split(r, 0, 6, true), "the focused field is never left out");
}
