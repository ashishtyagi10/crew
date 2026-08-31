//! What a toast card SAYS: its text clipped to the width it has, the repeat
//! count on its legend, and the fade as it leaves.
use super::*;
use crate::layout::Rect;
use crate::toast::{push_toasts, Toasts};
use crew_render::CellView;

#[test]
fn exit_window_fades_text_toward_the_page() {
    let _g = crate::app::theme_test_guard();
    // fade = 1 exactly at expiry: the text cell fg equals page_bg.
    let cells_mid = card_cells(
        &CardText {
            text: "hi",
            legend: "note",
            repeats: 1,
            alert: false,
            actionable: false,
        },
        6,
        0.0,
        false,
    );
    let cells_end = card_cells(
        &CardText {
            text: "hi",
            legend: "note",
            repeats: 1,
            alert: false,
            actionable: false,
        },
        6,
        1.0,
        false,
    );
    let t = crew_theme::theme();
    let text_cell = |cells: &Vec<crew_render::CellView>| {
        cells
            .iter()
            .find(|c| c.row == 1 && c.c == 'h')
            .expect("text cell")
            .fg
    };
    assert_eq!(text_cell(&cells_mid), t.ink);
    assert_eq!(text_cell(&cells_end), t.page_bg);
}

#[test]
fn long_text_clips_on_a_cell_boundary_with_ellipsis() {
    // The clip itself lives in `chatwidth::clip_w` (shared with every card
    // legend); here we assert the toast body actually goes through it.
    let mut t = Toasts::default();
    t.push("x".repeat(MAX_TEXT_COLS + 20), "note", false, 1_000);
    let mut scenes = Vec::new();
    let content = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    push_toasts(&mut scenes, &mut t, content, 8.0, 16.0, 2_000, None);
    let s = &scenes[0];
    assert_eq!(s.w, (MAX_TEXT_COLS + 4) as f32 * 8.0, "card caps its width");
    assert!(
        s.cells.iter().any(|c| c.row == 1 && c.c == '…'),
        "over-wide toast text must end in an ellipsis"
    );
}

/// The count is on the legend, and it survives the hover rewrite — the
/// reason you are hovering may well be that the card said it happened four
/// times.
#[test]
fn the_count_is_drawn_on_the_legend_and_survives_hover() {
    let _g = crate::app::theme_test_guard();
    let text_of = |cells: Vec<CellView>| -> String {
        let mut v: Vec<&CellView> = cells.iter().filter(|c| c.row == 0).collect();
        v.sort_by_key(|c| c.col);
        v.iter().map(|c| c.c).collect()
    };
    let once = text_of(card_cells(
        &CardText {
            text: "x",
            legend: "done",
            repeats: 1,
            alert: false,
            actionable: true,
        },
        30,
        0.0,
        false,
    ));
    assert!(
        !once.contains('\u{d7}'),
        "a first arrival counts nothing: {once:?}"
    );
    let four = text_of(card_cells(
        &CardText {
            text: "x",
            legend: "done",
            repeats: 4,
            alert: false,
            actionable: true,
        },
        30,
        0.0,
        false,
    ));
    assert!(four.contains("done \u{d7}4"), "legend was {four:?}");
    let hovered = text_of(card_cells(
        &CardText {
            text: "x",
            legend: "done",
            repeats: 4,
            alert: false,
            actionable: true,
        },
        30,
        0.0,
        true,
    ));
    assert!(
        hovered.contains("done \u{d7}4"),
        "hover kept the count: {hovered:?}"
    );
    assert!(hovered.contains("open"), "and still offers the click");
}
