//! The Inbox section's rows: two per thread, what each says, what a row means.
use super::*;

fn thread(id: &str, from: &str, subject: &str) -> Thread {
    Thread {
        id: id.into(),
        from: from.into(),
        subject: subject.into(),
        unread: true,
        ..Default::default()
    }
}

fn text(cells: &[CellView], row: u16) -> String {
    let mut r: Vec<_> = cells.iter().filter(|c| c.row == row).collect();
    r.sort_by_key(|c| c.col);
    r.iter().map(|c| c.c).collect()
}

#[test]
fn a_row_means_the_thread_drawn_on_it() {
    assert_eq!(item_at(0, 3), None, "the rule");
    assert_eq!(item_at(1, 3), Some(0));
    assert_eq!(item_at(2, 3), Some(0), "the subject row is the same thread");
    assert_eq!(item_at(3, 3), Some(1));
    assert_eq!(item_at(7, 3), None, "past the last thread");
    assert_eq!(fits(1 + 2 * 4), 4);
}

#[test]
fn each_thread_says_who_then_what() {
    let _g = crate::app::theme_test_guard();
    let card = Card::Count {
        unread: 2,
        newest: None,
    };
    let list = [
        thread("a", "Ann Lee <ann@x.org>", "Q4 plan"),
        thread("b", "bob@y.org", ""),
    ];
    let cells = cells(&card, &list, (0, 2), None, 0, (30, 20));
    assert!(text(&cells, 0).contains("2 unread"));
    assert!(text(&cells, 1).contains("Ann Lee"));
    assert!(text(&cells, 2).contains("Q4 plan"));
    assert!(text(&cells, 4).contains("(no subject)"));
}

#[test]
fn more_unread_than_rows_says_how_many_more() {
    let _g = crate::app::theme_test_guard();
    let card = Card::Count {
        unread: 40,
        newest: None,
    };
    let list: Vec<_> = (0..12).map(|i| thread(&i.to_string(), "a", "s")).collect();
    // Room for 3 threads (rows 1..=6) and the note on row 7.
    let cells = cells(&card, &list, (0, 3), None, 0, (30, 8));
    assert!(text(&cells, 7).contains("37 more"), "{}", text(&cells, 7));
}

#[test]
fn signed_out_says_so_instead_of_a_blank_panel() {
    let _g = crate::app::theme_test_guard();
    let cells = cells(&Card::Off, &[], (0, 0), None, 0, (30, 10));
    assert!(text(&cells, 1).contains("not signed in"));
    let none = Card::Count {
        unread: 0,
        newest: None,
    };
    assert!(text(&cells_of(&none), 1).contains("nothing unread"));
}

fn cells_of(card: &Card) -> Vec<CellView> {
    cells(card, &[], (0, 0), None, 0, (30, 10))
}
