//! `cmdmenu::popup`: the menu card cut to its own measure. A separate file
//! because `cmdmenu_tests.rs` sits at its line-cap debt.
use crate::cmdmenu::popup;
use crate::popupplace::MIN_COLS;
use crate::suggest::MenuItem;

fn item(label: &str, desc: &str) -> MenuItem {
    MenuItem {
        label: label.into(),
        desc: desc.into(),
        ..Default::default()
    }
}

/// One short row is still a card of the minimum width, not a sliver.
#[test]
fn a_short_list_is_still_a_card() {
    let p = popup("attach", &[item("@smith", "")], 0, 120);
    assert_eq!(p.cols, MIN_COLS);
    assert_eq!(p.rows, 3, "one row plus the frame");
}

/// A list wider than the floor hugs its own measure: the rows' width, one
/// column of air, and the two frame columns — well short of the pane.
#[test]
fn a_long_list_hugs_its_measure() {
    let items = [
        item("/dash", "Open the dashboard pane"),
        item(
            "/doctor",
            "Report what crew can reach, provider by provider",
        ),
    ];
    let measure = crate::cmdrow::content_w(&items);
    assert!(
        measure + 2 > usize::from(MIN_COLS),
        "the fixture is wider than the floor"
    );
    let p = popup("commands", &items, 0, 200);
    assert_eq!(usize::from(p.cols), measure + 3);
    assert_eq!(
        popup("commands", &items, 0, 30).cols,
        30,
        "never wider than the pane"
    );
}

/// The frame is drawn at the hugged width: its right border stands at the
/// card's last column, and no cell lies past it.
#[test]
fn the_frame_ends_at_the_hugged_width() {
    let items = [
        item("/dash", "Open the dashboard pane"),
        item(
            "/doctor",
            "Report what crew can reach, provider by provider",
        ),
    ];
    let p = popup("commands", &items, 1, 200);
    let right = p.cells.iter().map(|c| c.col).max().unwrap();
    assert_eq!(right, p.cols - 1);
    assert!(
        p.cells
            .iter()
            .any(|c| c.col == right && c.row == 1 && c.c == '│'),
        "a right border"
    );
    let text_end = p
        .cells
        .iter()
        .filter(|c| c.row == 2 && c.c != '│')
        .map(|c| c.col)
        .max();
    assert_eq!(
        text_end,
        Some(right - 2),
        "one column of air before the border"
    );
    assert!(
        p.cells
            .iter()
            .any(|c| c.col == 1 && c.row == 2 && c.c == '›'),
        "the list starts at the left edge"
    );
}

/// The measure is the widest ROW. A long label with no description (a file
/// path) and a long description on another row do not add up to a row that
/// is not there.
#[test]
fn the_measure_is_the_widest_row_not_the_sum_of_the_widest_parts() {
    let items = [
        item("@skill:verify", "skill · drive the live GUI"),
        item("@crates/crew-app/src/render.rs", ""),
    ];
    // marker + label column (13) + gap + desc (26)
    assert_eq!(crate::cmdrow::content_w(&items), 2 + 13 + 2 + 26);
    let path_only = [item("@crates/crew-app/src/render.rs", "")];
    assert_eq!(crate::cmdrow::content_w(&path_only), 2 + 30);
}

/// The selected row's description reads in the page's ink; every other
/// row's stays muted. A row the stack cannot serve keeps its mute even
/// when selected — that mute is the message.
#[test]
fn the_selected_row_reads_in_full_ink() {
    let _g = crate::app::theme_test_guard();
    let items = [
        item("/dash", "Open the dashboard pane"),
        item("/diff", "Review the working tree"),
        MenuItem {
            dim: true,
            ..item("/dead", "no provider on this stack")
        },
    ];
    let (ink, muted) = (crew_theme::theme().ink, crate::menuink::desc());
    let desc_fg = |cells: &[crew_render::CellView], row: u16| {
        cells
            .iter()
            .find(|c| c.row == row && c.c == 'e')
            .map(|c| c.fg)
    };
    let cells = crate::cmdmenu::menu_cells(&items, 1, 60, 3);
    assert_eq!(desc_fg(&cells, 1), Some(ink), "the selected row");
    assert_eq!(desc_fg(&cells, 0), Some(muted), "an unselected row");
    let cells = crate::cmdmenu::menu_cells(&items, 2, 60, 3);
    let label = cells
        .iter()
        .find(|c| c.row == 2 && c.c == '/')
        .map(|c| c.fg);
    assert_eq!(label, Some(muted), "a dim row stays muted when selected");
}

/// A list longer than the card shows says where in it you are.
#[test]
fn a_scrolling_list_marks_where_the_selection_is() {
    let _g = crate::app::theme_test_guard();
    let items: Vec<MenuItem> = (0..14)
        .map(|i| item(&format!("/c{i}"), "a command"))
        .collect();
    let top = |p: &crate::popupplace::Popup| -> String {
        p.cells.iter().filter(|c| c.row == 0).map(|c| c.c).collect()
    };
    assert!(top(&popup("commands", &items, 12, 80)).contains("13/14"));
    // Section titles are not choices: with two of them above the selection
    // the mark counts what you can actually land on.
    let mut titled: Vec<MenuItem> = (0..14)
        .map(|i| item(&format!("/c{i}"), "a command"))
        .collect();
    titled[0].header = true;
    titled[5].header = true;
    assert!(top(&popup("commands", &titled, 12, 80)).contains("11/12"));
    assert!(
        !top(&popup("commands", &items[..5], 2, 80)).contains('/'),
        "a short list has no mark"
    );
}

/// `offset` is what ratatui does with a fresh `ListState`: scrolls just far
/// enough to keep the selection on the last visible row.
#[test]
fn the_offset_is_the_one_ratatui_draws_with() {
    use ratatui::widgets::{List, ListItem, ListState, StatefulWidget};
    for (n, sel) in [(14, 12), (14, 0), (14, 13), (5, 4), (30, 9), (30, 10)] {
        let items: Vec<ListItem> = (0..n).map(|i| ListItem::new(format!("r{i}"))).collect();
        let rows = crate::cmdmenu::menu_rows(n) - 2;
        let mut buf = ratatui::buffer::Buffer::empty(ratatui::layout::Rect::new(0, 0, 20, rows));
        let mut state = ListState::default();
        state.select(Some(sel));
        StatefulWidget::render(List::new(items), buf.area, &mut buf, &mut state);
        assert_eq!(
            crate::cmdmenu::offset(n, sel),
            state.offset(),
            "n={n} sel={sel}"
        );
    }
}
