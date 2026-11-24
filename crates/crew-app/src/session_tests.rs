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

#[test]
fn ctrl_digits_and_slash_follow_xterms_table() {
    assert_eq!(ctrl_byte('2'), Some(0x00));
    assert_eq!(ctrl_byte('3'), Some(0x1b));
    assert_eq!(ctrl_byte('4'), Some(0x1c));
    assert_eq!(ctrl_byte('7'), Some(0x1f));
    assert_eq!(ctrl_byte('8'), Some(0x7f));
    assert_eq!(ctrl_byte('/'), Some(0x1f));
    assert_eq!(ctrl_byte('9'), None);
    assert_eq!(ctrl_byte('0'), None);
}

/// ´ then e: winit folds the accent into `text` while the logical key stays
/// the keycap's `e`, and the pane got the bare letter.
#[test]
fn printable_keys_type_the_composed_text() {
    use winit::keyboard::{Key, SmolStr};
    let e = Key::Character(SmolStr::new("e"));
    assert_eq!(typed_bytes(&e, Some("é"), false).unwrap(), [0xc3, 0xa9]);
    let a = Key::Character(SmolStr::new("a"));
    assert_eq!(typed_bytes(&a, Some("a"), false).unwrap(), b"a");
    assert_eq!(typed_bytes(&a, None, false).unwrap(), b"a");
    // The accent alone has nothing to type yet.
    assert_eq!(typed_bytes(&Key::Dead(Some('´')), None, false), None);
    // Ctrl reads the keycap, whatever text came with it.
    let c = Key::Character(SmolStr::new("c"));
    assert_eq!(typed_bytes(&c, Some("\u{3}"), true).unwrap(), [0x03]);
    assert_eq!(typed_bytes(&c, Some("ç"), true).unwrap(), [0x03]);
}

/// Option as Meta on a Mac is decided before any text is typed: Option+e is
/// still `ESC e`, not the accent.
#[test]
fn option_as_meta_still_sends_escape_e() {
    let alt = crate::keyenc::Mods {
        alt: true,
        ..Default::default()
    };
    let meta = crate::altmeta::meta("´", "e", alt, true, true);
    assert_eq!(meta.unwrap(), b"\x1be");
}
