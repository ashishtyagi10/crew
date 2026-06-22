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
    let cells = layout_cells(&[msg("a", "hi")], "xy", 20, 5, 0);
    assert!(cells.iter().any(|c| c.c == '>' && c.row == 4 && c.col == 0));
}

#[test]
fn layout_cells_input_chars_on_last_row() {
    let cells = layout_cells(&[msg("a", "hi")], "xy", 20, 5, 0);
    assert!(find(&cells, 'x', 4), "expected 'x' on row 4");
    assert!(find(&cells, 'y', 4), "expected 'y' on row 4");
}

#[test]
fn layout_cells_message_above_prompt() {
    let cells = layout_cells(&[msg("a", "hi")], "xy", 20, 5, 0);
    assert!(cells.iter().any(|c| c.c == 'a' && c.row < 4));
}

#[test]
fn layout_cells_sender_in_accent_fg() {
    let cells = layout_cells(&[msg("bob", "hello")], "", 20, 3, 0);
    assert!(cells.iter().any(|c| c.fg == ACCENT_FG));
}

#[test]
fn layout_cells_empty_msgs_prompt_only() {
    let cells = layout_cells(&[], "", 10, 2, 0);
    assert!(cells.iter().any(|c| c.c == '>' && c.row == 1));
}

#[test]
fn layout_cells_zero_rows_returns_empty() {
    assert!(layout_cells(&[], "", 10, 0, 0).is_empty());
}

#[test]
fn wrapped_line_count_counts_wrapping() {
    // "a: " + 18 chars = 21 chars, wraps to 3 lines at width 10.
    let m = msg("a", &"x".repeat(18));
    assert_eq!(wrapped_line_count(std::slice::from_ref(&m), 10), 3);
}

#[test]
fn scrolling_up_reveals_an_older_message() {
    let msgs: Vec<Message> = (0..10).map(|i| msg("s", &format!("M{i}"))).collect();
    // 3 rows → 2 message rows; at the bottom, "M0" is hidden.
    assert!(!layout_cells(&msgs, "", 20, 3, 0).iter().any(|c| c.c == '0'));
    // scrolling up far enough brings it back into view.
    assert!(layout_cells(&msgs, "", 20, 3, 8).iter().any(|c| c.c == '0'));
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
