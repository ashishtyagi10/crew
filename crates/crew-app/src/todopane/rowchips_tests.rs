use super::{chips, has_chips, row_chips, RowCtx};
use crate::todopane::item::TodoItem;
use crate::todopane::test_pane;

fn item(title: &str, project: Option<&str>, who: Option<&str>) -> TodoItem {
    TodoItem {
        id: 1,
        title: title.into(),
        project: project.map(str::to_string),
        assignee: who.map(str::to_string),
        created_ms: 1,
        ..Default::default()
    }
}

const FLAT: RowCtx = RowCtx {
    done_view: false,
    banded: false,
};
const BANDED: RowCtx = RowCtx {
    done_view: false,
    banded: true,
};

/// The band header names the owner; repeating it on every row under it is
/// the same word three times, and it is the chip that costs the width.
#[test]
fn a_banded_row_drops_the_owner_and_keeps_the_project() {
    let it = item("renew the domain", Some("admin"), Some("sam"));
    assert_eq!(row_chips(&it, FLAT), vec!["@admin", "#sam"]);
    assert_eq!(
        row_chips(&it, BANDED),
        vec!["@admin"],
        "the band says #sam already; @admin it does not say"
    );
}

/// The round-trip list is a different question from the drawn one: `e` on a
/// row under its own band must reload the owner, or resubmitting unchanged
/// would file the item back belonging to nobody.
#[test]
fn the_round_trip_list_keeps_the_owner_whatever_the_list_is_doing() {
    let it = item("renew the domain", Some("admin"), Some("sam"));
    assert_eq!(chips(&it), vec!["@admin", "#sam"]);
}

/// An item whose only right-side content was the owner has nothing left to
/// stack under a band — the height and the draw must agree about that.
#[test]
fn an_owner_only_row_carries_nothing_on_its_right_inside_a_band() {
    let it = item("reply to the invoice thread", None, Some("sam"));
    assert!(
        has_chips(&it, FLAT),
        "flat, the #sam chip is its right side"
    );
    assert!(
        !has_chips(&it, BANDED),
        "banded, that chip is gone and the row has no right side at all"
    );
}

/// `rowctx` reads the LIST, not the row: banding is on for every row or
/// none, and it is off the moment the bands are (nobody named, or `g`).
#[test]
fn rowctx_follows_the_lists_bands() {
    let named = vec![item("a", None, Some("sam"))];
    let p = test_pane(named.clone());
    assert!(p.grouped && p.rowctx().banded, "a named list bands");

    let mut flat = test_pane(named);
    flat.grouped = false;
    assert!(
        !flat.rowctx().banded,
        "g lays it flat, so the chips come back"
    );

    let nobody = test_pane(vec![item("a", Some("admin"), None)]);
    assert!(
        !nobody.rowctx().banded,
        "nobody named means no bands, so nothing to drop"
    );

    let mut done = test_pane(vec![item("a", None, Some("sam"))]);
    done.set_done_view(true);
    assert!(
        !done.rowctx().banded && done.rowctx().done_view,
        "the history bands by DAY, so its rows still have to name the owner"
    );
}

/// The text of one drawn row, cells sorted by column, gaps as spaces.
fn row_text(cells: &[crew_render::CellView], row: u16) -> String {
    let mut on: Vec<&crew_render::CellView> = cells.iter().filter(|c| c.row == row).collect();
    on.sort_by_key(|c| c.col);
    let mut s = String::new();
    for c in on {
        while (s.chars().count() as u16) < c.col {
            s.push(' ');
        }
        s.push(c.c);
    }
    s
}

/// The DRAWN row, not just the chip list: under a `#sam` band the name is on
/// the header and nowhere else, while the `@project` the band does not name
/// stays. Flat, the row carries both again.
#[test]
fn a_banded_row_prints_the_owner_on_the_header_only() {
    let mut p = test_pane(vec![item("renew the domain", Some("admin"), Some("sam"))]);
    let banded = crate::todopane::render::cells(&p, 60, 20);
    assert!(
        row_text(&banded, 0).contains("#sam"),
        "the band header names him"
    );
    let row = row_text(&banded, 1);
    assert!(
        row.contains("renew the domain") && row.contains("@admin"),
        "{row}"
    );
    assert!(
        !row.contains("#sam"),
        "the row repeats a name the header one row above already gave: {row}"
    );

    p.grouped = false;
    let row = row_text(&crate::todopane::render::cells(&p, 60, 20), 0);
    assert!(
        row.contains("#sam") && row.contains("@admin"),
        "flat there is no header, so the row has to carry the owner: {row}"
    );
}
