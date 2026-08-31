use super::sidebar_pane_index;

#[test]
fn sidebar_rows_map_to_pane_indices() {
    let top = 21; // content row of the PANES header
                  // Border row 0 … header (outer row 22) → no pane.
    assert_eq!(sidebar_pane_index(0, top), None);
    assert_eq!(sidebar_pane_index(top + 1, top), None, "header row");
    // First pane row sits directly under the header.
    let first = top + 2;
    assert_eq!(sidebar_pane_index(first, top), Some(0));
    assert_eq!(sidebar_pane_index(first + 1, top), Some(1));
    assert_eq!(sidebar_pane_index(first + 2, top), Some(2));
}
