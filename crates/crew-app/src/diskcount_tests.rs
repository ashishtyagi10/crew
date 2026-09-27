//! The /disk header counts in words that agree with the number.

use crate::diskpane::DiskPane;

fn header(p: &DiskPane) -> String {
    let mut v: Vec<_> = p.cells(80, 20).into_iter().filter(|c| c.row == 0).collect();
    v.sort_by_key(|c| c.col);
    v.iter().map(|c| c.c).collect()
}

#[test]
fn a_folder_holding_one_thing_holds_one_entry() {
    let _g = crate::app::theme_test_guard();
    let mut p = DiskPane::new(std::path::PathBuf::from("/tmp/one"));
    p.set_children_for_test(&[("only", 4_096, false)], 0);
    let row = header(&p);
    assert!(row.contains("in 1 entry"), "{row:?}");
    assert!(!row.contains("entries"), "{row:?}");
    p.set_children_for_test(&[("a", 4_096, false), ("b", 1, false)], 0);
    assert!(header(&p).contains("in 2 entries"), "{:?}", header(&p));
}
