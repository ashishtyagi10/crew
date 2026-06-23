use super::*;

fn msg(sender: &str, text: &str) -> Message {
    Message {
        sender: sender.into(),
        text: text.into(),
    }
}

fn find(cells: &[CellView], ch: char, row: u16) -> bool {
    cells.iter().any(|c| c.c == ch && c.row == row)
}

#[test]
fn layout_cells_prompt_on_last_row() {
    let cells = layout_cells(&[msg("a", "hi")], "xy", 20, 5, 0, true);
    assert!(cells.iter().any(|c| c.c == '>' && c.row == 4 && c.col == 0));
}

#[test]
fn empty_pane_shows_dim_hint_no_messages_does_not() {
    // No messages → dim getting-started hint on row 0.
    let empty = layout_cells(&[], "", 60, 6, 0, true);
    assert!(empty
        .iter()
        .any(|c| c.c == 'T' && c.row == 0 && c.fg == HINT_FG));
    // Once a message exists, the hint is gone (message text occupies row 0).
    let with_msg = layout_cells(&[msg("agent", "hello")], "", 60, 6, 0, true);
    assert!(!with_msg.iter().any(|c| c.fg == HINT_FG));
}

#[test]
fn empty_pane_hint_reflects_connection_state() {
    // Connected → the "Type a message…" hint (starts with 'T').
    let ready = layout_cells(&[], "", 60, 6, 0, true);
    assert!(ready
        .iter()
        .any(|c| c.c == 'T' && c.row == 0 && c.fg == HINT_FG));
    // Not connected → the "Connecting…" hint (starts with 'C'), and no 'T' hint.
    let connecting = layout_cells(&[], "", 60, 6, 0, false);
    assert!(connecting
        .iter()
        .any(|c| c.c == 'C' && c.row == 0 && c.fg == HINT_FG));
    assert!(!connecting.iter().any(|c| c.c == 'T' && c.fg == HINT_FG));
}

#[test]
fn layout_cells_input_chars_on_last_row() {
    let cells = layout_cells(&[msg("a", "hi")], "xy", 20, 5, 0, true);
    assert!(find(&cells, 'x', 4), "expected 'x' on row 4");
    assert!(find(&cells, 'y', 4), "expected 'y' on row 4");
}

#[test]
fn layout_cells_message_above_prompt() {
    let cells = layout_cells(&[msg("a", "hi")], "xy", 20, 5, 0, true);
    assert!(cells.iter().any(|c| c.c == 'a' && c.row < 4));
}

#[test]
fn layout_cells_sender_in_accent_fg() {
    let cells = layout_cells(&[msg("bob", "hello")], "", 20, 3, 0, true);
    assert!(cells.iter().any(|c| c.fg == ACCENT_FG));
}

#[test]
fn layout_cells_empty_msgs_prompt_only() {
    let cells = layout_cells(&[], "", 10, 2, 0, true);
    assert!(cells.iter().any(|c| c.c == '>' && c.row == 1));
}

#[test]
fn layout_cells_zero_rows_returns_empty() {
    assert!(layout_cells(&[], "", 10, 0, 0, true).is_empty());
}

#[test]
fn wrapped_line_count_counts_wrapping() {
    // "a: " + 18 chars = 21 chars, wraps to 3 lines at width 10.
    let m = msg("a", &"x".repeat(18));
    assert_eq!(wrapped_line_count(std::slice::from_ref(&m), 10), 3);
}

#[test]
fn wrapping_breaks_at_word_boundaries() {
    // "s: alpha beta gamma" (19 chars) at width 12 should break between words,
    // not mid-word: "s: alpha" | "beta gamma".
    let m = msg("s", "alpha beta gamma");
    assert_eq!(wrapped_line_count(std::slice::from_ref(&m), 12), 2);
    let cells = layout_cells(std::slice::from_ref(&m), "", 12, 4, 0, true);
    // no rendered row contains a word split across its edge: the first content
    // row ends with "alpha" intact (its last char 'a' present, and "beta" starts
    // the next row).
    let row0: String = {
        let mut v: Vec<(u16, char)> = cells
            .iter()
            .filter(|c| c.row == 0)
            .map(|c| (c.col, c.c))
            .collect();
        v.sort();
        v.into_iter().map(|(_, c)| c).collect()
    };
    assert_eq!(row0.trim_end(), "s: alpha");
}

#[test]
fn scrolling_up_reveals_an_older_message() {
    let msgs: Vec<Message> = (0..10).map(|i| msg("s", &format!("M{i}"))).collect();
    // 3 rows → 2 message rows; at the bottom, "M0" is hidden.
    assert!(!layout_cells(&msgs, "", 20, 3, 0, true)
        .iter()
        .any(|c| c.c == '0'));
    // scrolling up far enough brings it back into view.
    assert!(layout_cells(&msgs, "", 20, 3, 8, true)
        .iter()
        .any(|c| c.c == '0'));
}

#[test]
fn input_reduce_pushes_char() {
    let mut s = String::new();
    assert_eq!(input_reduce(&mut s, Some('z'), false, false), None);
    assert_eq!(s, "z");
}

#[test]
fn input_reduce_enter_returns_and_clears() {
    let mut s = "hello".to_string();
    assert_eq!(
        input_reduce(&mut s, None, true, false),
        Some("hello".to_string())
    );
    assert_eq!(s, "");
}

#[test]
fn input_reduce_enter_empty() {
    let mut s = String::new();
    assert_eq!(
        input_reduce(&mut s, None, true, false),
        Some("".to_string())
    );
}

#[test]
fn input_reduce_backspace_pops() {
    let mut s = "abc".to_string();
    assert_eq!(input_reduce(&mut s, None, false, true), None);
    assert_eq!(s, "ab");
}

#[test]
fn input_reduce_backspace_empty_noop() {
    let mut s = String::new();
    assert_eq!(input_reduce(&mut s, None, false, true), None);
    assert_eq!(s, "");
}

#[test]
fn input_reduce_control_char_ignored() {
    let mut s = String::new();
    input_reduce(&mut s, Some('\n'), false, false);
    assert_eq!(s, "");
}
