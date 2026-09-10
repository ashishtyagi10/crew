//! The `/keys` overlay's chrome: the frame it shares with the composer
//! pop-ups, the hint on its bottom edge, the heading over the global keys.
//! A separate file because `help_tests.rs` sits at its line-cap debt.
use crate::help::{help_cells, size};
use crew_render::CellView;

fn row(cells: &[CellView], r: u16) -> String {
    let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == r).collect();
    v.sort_by_key(|c| c.col);
    // Later cells overdraw earlier ones at the same column, as the frame does.
    let mut out: Vec<(u16, char)> = Vec::new();
    for c in v {
        match out.iter_mut().find(|(col, _)| *col == c.col) {
            Some(slot) => slot.1 = c.c,
            None => out.push((c.col, c.c)),
        }
    }
    out.into_iter().map(|(_, c)| c).collect()
}

/// The frame is the composer pop-ups' — the focused stroke at the corners,
/// the legend bold in the accent — not a flat accent border of its own.
#[test]
fn the_overlay_wears_the_popup_chrome() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = size();
    let cells = help_cells(w, h, 0, "");
    let frame = crate::popupchrome::card(w, h, "keys");
    let corner = |v: &[CellView]| {
        v.iter()
            .rev()
            .find(|c| c.row == h - 1 && c.col == 0)
            .map(|c| c.fg)
    };
    assert_eq!(
        corner(&cells),
        corner(&frame),
        "the same stroke as a pop-up"
    );
    let legend: Vec<&CellView> = cells
        .iter()
        .filter(|c| c.row == 0 && c.c.is_alphabetic())
        .collect();
    assert!(legend.len() >= "keys".len());
    assert!(
        legend
            .iter()
            .all(|c| c.bold && c.fg == crate::palette::accent()),
        "bold accent legend"
    );
    assert!(row(&cells, 0).contains(&format!("keys \u{b7} crew v{}", env!("CARGO_PKG_VERSION"))));
}

/// The hint rides the bottom border, right-aligned two columns in, in the
/// muted ink — the frame stays the frame around it.
#[test]
fn the_hint_rides_the_bottom_border_at_the_right() {
    let _g = crate::app::theme_test_guard();
    let (w, _) = size();
    let cells = help_cells(w, 24, 0, "");
    let bottom = row(&cells, 23);
    let hint = " \u{2191}\u{2193} for more \u{b7} type to filter \u{b7} esc ";
    let at = bottom.find(hint).expect("the hint is on the bottom row");
    let at = bottom[..at].chars().count();
    assert_eq!(
        at + hint.chars().count() + 2,
        usize::from(w),
        "two columns in"
    );
    assert!(bottom.ends_with('\u{256f}'), "{bottom:?}");
    let muted = crew_theme::theme().text_muted;
    let f: Vec<&CellView> = cells.iter().filter(|c| c.row == 23 && c.c == 'f').collect();
    assert!(!f.is_empty() && f.iter().all(|c| c.fg == muted));
}

/// The global keys sit under a heading of their own, ruled like the rest.
/// (The blank after the word is the opaque filler `to_cells_opaque` draws.)
#[test]
fn the_global_keys_have_a_ruled_heading() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = size();
    let cells = help_cells(w, h, 0, "");
    let first = row(&cells, 1);
    assert!(first.starts_with("\u{2502}everywhere"), "{first:?}");
    assert!(
        first.ends_with("\u{2500}\u{2500}\u{2500}\u{2502}"),
        "ruled to the edge: {first:?}"
    );
}
