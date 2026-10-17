//! A palette capped at the pane keeps the air on its right. Split from
//! `cmdmenu_tests` and `cmdmenupopup_tests`, which sit at the line cap.
use crate::cmdmenu::menu_cells;
use crate::suggest::MenuItem;

fn item(label: &str, desc: &str) -> MenuItem {
    MenuItem {
        label: label.into(),
        desc: desc.into(),
        ..Default::default()
    }
}

/// Survey H2: at a quarter tile the card is capped at the pane, and the rows
/// ran into the last interior column; the `…` of a cut description sat 3 px
/// off the stroke while the left kept 21.
#[test]
fn a_capped_palette_leaves_its_last_two_columns_blank() {
    let _g = crate::app::theme_test_guard();
    let items = [
        item(
            "/approvals",
            "approval mode: auto/edits/ask/plan/yolo, and what each one lets an agent do",
        ),
        item(
            "/reload",
            "re-read skills, plugin agents and the MCP config from disk",
        ),
    ];
    for cols in [30u16, 46, 60] {
        let cells = menu_cells(&items, 0, cols, 2);
        let inked = |c: &&crew_render::CellView| c.c != ' ';
        let right = cells.iter().filter(inked).map(|c| c.col).max().unwrap();
        assert!(right < cols - 2, "{cols} columns: ink at {right}");
    }
}

/// A card the rows fit in is drawn exactly as before: the rows' own measure.
#[test]
fn a_card_with_room_lays_its_rows_at_their_measure() {
    let _g = crate::app::theme_test_guard();
    let items = [item("/dash", "Open the dashboard pane")];
    let w = crate::cmdrow::content_w(&items) as u16;
    let cells = menu_cells(&items, 0, w + 2, 1);
    let right = cells
        .iter()
        .filter(|c| c.c != ' ')
        .map(|c| c.col)
        .max()
        .unwrap();
    assert!(right < w, "ink stays inside the measure: {right} vs {w}");
}
