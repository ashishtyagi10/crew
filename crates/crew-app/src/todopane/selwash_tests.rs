use crate::todopane::item::TodoItem;
use crate::todopane::render::{cells, click_at, TodoClick, TITLE_COL};
use crate::todopane::test_pane;

fn item(id: u64, title: &str, project: Option<&str>) -> TodoItem {
    TodoItem {
        id,
        title: title.to_string(),
        project: project.map(str::to_string),
        created_ms: id,
        ..Default::default()
    }
}

/// At a narrow width the selected item wraps and stacks its chip row; every
/// one of its rows is washed from the edge to the column before the gutter,
/// and no other row carries the wash.
#[test]
fn the_selected_item_is_washed_as_one_block() {
    let _g = crate::app::theme_test_guard();
    let (cols, rows) = (34u16, 20u16);
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        let wash = crew_theme::readable::selection_bg(&t);
        let mut p = test_pane(vec![
            item(1, "shoot the whole chat pane", Some("crew")),
            item(2, "renumber the unified diff from the file", Some("viewer")),
            item(3, "read the blame gutter runs", None),
        ]);
        p.sel = Some(1);
        let out = cells(&p, cols, rows);
        let mine: Vec<u16> = (0..rows)
            .filter(|&r| click_at(&p, r, TITLE_COL, cols, rows) == Some(TodoClick::Select(1)))
            .collect();
        assert!(mine.len() >= 2, "{id:?}: the item should wrap: {mine:?}");
        for &r in &mine {
            for col in 0..cols - 1 {
                let c = out.iter().rev().find(|c| c.row == r && c.col == col);
                assert_eq!(c.map(|c| c.bg), Some(wash), "{id:?} row {r} col {col}");
            }
        }
        let stray = out
            .iter()
            .map(|c| (c.row, c.col, c.c))
            .zip(out.iter())
            .find(|(_, c)| c.bg == wash && !mine.contains(&c.row));
        assert!(
            stray.is_none(),
            "{id:?}: washed off the item: {:?}",
            stray.map(|s| s.0)
        );
    }
}
