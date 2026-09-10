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
