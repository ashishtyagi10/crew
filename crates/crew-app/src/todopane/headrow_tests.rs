use crate::todopane::item::TodoItem;
use crate::todopane::render::cells;
use crate::todopane::test_pane;

const COLS: u16 = 70;
const ROWS: u16 = 20;

fn item(id: u64, title: &str, who: Option<&str>, project: Option<&str>) -> TodoItem {
    TodoItem {
        id,
        title: title.to_string(),
        assignee: who.map(str::to_string),
        project: project.map(str::to_string),
        created_ms: id,
        ..Default::default()
    }
}

/// The text of one row, cells sorted by column, gaps as spaces.
fn row_text(cells: &[crew_render::CellView], row: u16) -> String {
    let mut on_row: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == row).collect();
    on_row.sort_by_key(|c| c.col);
    let mut s = String::new();
    let mut x = 0;
    for c in on_row {
        while x < c.col {
            s.push(' ');
            x += 1;
        }
        s.push(c.c);
        x += 1;
    }
    s
}

fn grouped() -> crate::todopane::TodoPane {
    let now = crate::chattime::unix_now_ms();
    let mut items = vec![
        item(1, "ship notes", Some("priya"), Some("crew")),
        item(2, "review it", Some("priya"), None),
        item(3, "wire hook", Some("sam"), None),
        item(4, "pay rent", None, Some("home")),
    ];
    items[1].due_ms = Some(now - 3_600_000); // priya has one overdue
    let mut p = test_pane(items);
    p.grouped = true;
    p
}

#[test]
fn a_grouped_list_bands_every_person_with_a_roll_up() {
    let _g = crate::app::theme_test_guard();
    let p = grouped();
    let out = cells(&p, COLS, ROWS);
    let rows: Vec<String> = (0..8).map(|r| row_text(&out, r)).collect();
    let joined = rows.join("\n");
    assert!(
        joined.contains("#priya  2 open \u{b7} 1 overdue \u{b7} 0 done today"),
        "{joined}"
    );
    assert!(
        joined.contains("#sam  1 open \u{b7} 0 overdue \u{b7} 0 done today"),
        "{joined}"
    );
    // The bucket nobody has picked up is NAMED, or a grouped list just
    // stops having headers at the bottom and reads as having ended.
    assert!(joined.contains("unassigned  1 open"), "{joined}");
    // Header rows carry no checkbox — they are not items.
    // A band header is not an item row: no checkbox, and exactly one of
    // them per person however many items that person carries.
    let heads: Vec<&String> = rows
        .iter()
        .filter(|r| r.contains("#priya") && !r.contains("[ ]"))
        .collect();
    assert_eq!(heads.len(), 1, "one band per person: {joined}");
}

#[test]
fn a_row_wears_its_owner_and_its_project_side_by_side() {
    let _g = crate::app::theme_test_guard();
    let mut p = grouped();
    p.grouped = false;
    let out = cells(&p, COLS, ROWS);
    let row = (0..8)
        .map(|r| row_text(&out, r))
        .find(|r| r.contains("ship notes"))
        .expect("the item is on screen");
    let (who, proj) = (row.find("#priya"), row.find("@crew"));
    assert!(who.is_some() && proj.is_some(), "{row:?}");
    assert!(
        who < proj,
        "the owner sits nearer the title than the project: {row:?}"
    );
}

#[test]
fn a_flat_list_draws_no_bands_at_all() {
    let _g = crate::app::theme_test_guard();
    let mut p = grouped();
    p.grouped = false;
    let out = cells(&p, COLS, ROWS);
    let joined: String = (0..8)
        .map(|r| row_text(&out, r))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!joined.contains("open \u{b7}"), "no roll-up: {joined}");
    assert!(!joined.contains("unassigned"), "no buckets: {joined}");
}
