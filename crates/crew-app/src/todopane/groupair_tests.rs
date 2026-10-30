//! A row of air between people's bands, none above the first, and the
//! height math, the draw and the clicks agreeing on it.
use super::head_h;
use crate::todopane::item::TodoItem;
use crate::todopane::render::{cells, click_at, TodoClick};
use crate::todopane::test_pane;

fn item(id: u64, who: Option<&str>) -> TodoItem {
    TodoItem {
        id,
        title: format!("t{id}"),
        assignee: who.map(str::to_string),
        created_ms: id,
        ..Default::default()
    }
}

#[test]
fn people_bands_after_the_first_get_a_row_of_air() {
    let mut p = test_pane(vec![item(1, Some("priya")), item(2, Some("sam"))]);
    p.grouped = true;
    let ord = p.order();
    assert_eq!((head_h(&p, &ord, 0), head_h(&p, &ord, 1)), (1, 2));
    let rows: Vec<String> = (0..6)
        .map(|r| {
            let mut cs: Vec<_> = cells(&p, 80, 20)
                .into_iter()
                .filter(|c| c.row == r)
                .collect();
            cs.sort_by_key(|c| c.col);
            cs.iter()
                .map(|c| c.c)
                .collect::<String>()
                .trim()
                .to_string()
        })
        .collect();
    let at = |s: &str| rows.iter().position(|r| r.contains(s)).unwrap_or(99);
    let (priya, t1, sam) = (at("#priya"), at("t1"), at("#sam"));
    assert_eq!((t1, sam), (priya + 1, t1 + 2), "air between: {rows:#?}");
    assert!(rows[t1 + 1].is_empty(), "{rows:#?}");
    let air = (t1 + 1) as u16;
    assert_eq!(click_at(&p, air, 10, 80, 20), None, "the air is no row");
    assert_eq!(
        click_at(&p, air + 2, 10, 80, 20),
        Some(TodoClick::Select(1)),
        "sam's item, under its header"
    );
}
