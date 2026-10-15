//! Where the chat's status row ends, against the column the rows under it
//! keep.
use crate::chat::ChatPane;
use crew_plugin::Plugin;

/// The status row's last glyph stands in the column code blocks stop
/// short of, not on the pane's last: the `●` reached 7 px past every row
/// under it (glass survey F9).
#[test]
fn the_status_row_ends_a_column_in() {
    let sh = ["-c".to_string(), "cat >/dev/null".to_string()];
    let mut pane = ChatPane::new(Plugin::spawn("sh", &sh).unwrap(), "crew".into());
    pane.connected = true;
    for cols in [40u16, 72] {
        let cells = super::cells(&pane, cols, 20);
        let last = cells
            .iter()
            .filter(|c| c.row == 0 && c.c != ' ')
            .map(|c| c.col)
            .max();
        assert_eq!(last, Some(cols - 2), "at {cols} columns");
    }
}
