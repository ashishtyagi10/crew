use super::*;
use crate::layout::pane_rects_at;

#[test]
fn pane_at_two_panes() {
    // 2 panes side-by-side in 800x600 with no gap → left pane [0,400) right [400,800)
    let rects = pane_rects_at(2, 0.0, 0.0, 800.0, 600.0, 0.0);
    assert_eq!(pane_at(&rects, 10.0, 10.0), Some(0));
    assert_eq!(pane_at(&rects, 410.0, 10.0), Some(1));
    assert_eq!(pane_at(&rects, 800.0, 10.0), None);
}

#[test]
fn grid_for_basic() {
    let g = grid_for(800, 600, 10.0, 20.0);
    assert_eq!(g.cols, 80);
    assert_eq!(g.rows, 30);
}

#[test]
fn grid_for_clamps_to_one() {
    let g = grid_for(0, 0, 10.0, 20.0);
    assert_eq!(g.cols, 1);
    assert_eq!(g.rows, 1);
}

#[test]
fn grid_for_floors_partial_cells() {
    // 805 / 10 = 80.5 → floor → 80
    let g = grid_for(805, 601, 10.0, 20.0);
    assert_eq!(g.cols, 80);
    assert_eq!(g.rows, 30);
}

#[test]
fn arrow_keys_map_to_escape_sequences() {
    assert_eq!(named_bytes(NamedKey::ArrowUp).unwrap(), b"\x1b[A");
    assert_eq!(named_bytes(NamedKey::ArrowDown).unwrap(), b"\x1b[B");
    assert_eq!(named_bytes(NamedKey::ArrowRight).unwrap(), b"\x1b[C");
    assert_eq!(named_bytes(NamedKey::ArrowLeft).unwrap(), b"\x1b[D");
}

#[test]
fn nav_and_edit_keys_mapped() {
    assert_eq!(named_bytes(NamedKey::PageUp).unwrap(), b"\x1b[5~");
    assert_eq!(named_bytes(NamedKey::Delete).unwrap(), b"\x1b[3~");
    assert_eq!(named_bytes(NamedKey::Home).unwrap(), b"\x1b[H");
}

#[test]
fn wrap_paste_normalizes_and_brackets() {
    assert_eq!(wrap_paste("ab", false), b"ab");
    assert_eq!(wrap_paste("a\r\nb\nc", false), b"a\rb\rc");
    let w = wrap_paste("x", true);
    assert!(w.starts_with(b"\x1b[200~") && w.ends_with(b"\x1b[201~"));
}

#[test]
fn shift_enter_is_line_feed_plain_enter_is_return() {
    assert_eq!(named_bytes_shift(NamedKey::Enter, true).unwrap(), b"\n");
    assert_eq!(named_bytes_shift(NamedKey::Enter, false).unwrap(), b"\r");
    // Shift+Tab is still backtab; unshifted Tab is a plain tab.
    assert_eq!(named_bytes_shift(NamedKey::Tab, true).unwrap(), b"\x1b[Z");
    assert_eq!(named_bytes_shift(NamedKey::Tab, false).unwrap(), b"\t");
}

#[test]
fn ctrl_letters_become_control_codes() {
    assert_eq!(ctrl_byte('c'), Some(0x03));
    assert_eq!(ctrl_byte('C'), Some(0x03));
    assert_eq!(ctrl_byte('a'), Some(0x01));
    assert_eq!(ctrl_byte('1'), None);
}
