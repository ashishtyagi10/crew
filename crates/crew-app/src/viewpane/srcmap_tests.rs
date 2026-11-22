//! The prepared text's bytes, taken back to the file's.
use crate::viewpane::whitespace::prepare;

/// Every byte of the prepared text that maps back holds the same character in the file.
fn every_mapped_byte_holds_its_char(src: &str, reveal: bool) {
    let p = prepare(src, reveal);
    for (at, c) in p.text.char_indices() {
        let Some(o) = p.map.to_orig(at as u32) else {
            continue;
        };
        assert_eq!(
            src[o as usize..].chars().next(),
            Some(c),
            "prepared byte {at} ({c:?}) mapped to file byte {o}\n{src:?}"
        );
    }
}

#[test]
fn a_file_with_nothing_to_rewrite_is_the_identity() {
    let p = prepare("plain text\nand more\n", false);
    assert!(p.map.is_identity());
    assert_eq!(p.map.to_orig(5), Some(5));
    assert_eq!(p.map.to_prep(5), 5);
}

/// A CRLF file drops a byte per line. Without the map the caret on line ten splices nine
/// bytes early.
#[test]
fn a_crlf_file_maps_every_line_back() {
    let src = "one\r\ntwo\r\nthree\r\n";
    every_mapped_byte_holds_its_char(src, false);
    let p = prepare(src, false);
    let t = p.text.find("three").unwrap() as u32;
    assert_eq!(p.map.to_orig(t), Some(src.find("three").unwrap() as u32));
}

#[test]
fn an_expanded_tab_maps_its_spaces_to_nothing_and_what_follows_exactly() {
    let src = "a\tb\n\tc\n";
    every_mapped_byte_holds_its_char(src, false);
    let p = prepare(src, false);
    assert_eq!(
        p.map.to_orig(1),
        None,
        "the tab's first space is not a byte of the file"
    );
    let b = p.text.find('b').unwrap() as u32;
    assert_eq!(p.map.to_orig(b), Some(2));
    // …and the way back lands on the same character.
    assert_eq!(p.map.to_prep(2), b);
    assert_eq!(
        p.map.to_prep(1),
        1,
        "inside the tab: where its spaces start"
    );
}

#[test]
fn revealed_invisibles_map_back_too() {
    every_mapped_byte_holds_its_char("a\tb  \r\nc \n", true);
}

/// Round trip: every byte of the file that is a character of its own comes back to itself.
#[test]
fn to_prep_then_to_orig_is_the_identity_off_the_substitutions() {
    let src = "x\ty\r\nzz\t\r\nend\n";
    let p = prepare(src, false);
    for (o, c) in src.char_indices() {
        if c == '\t' || c == '\r' {
            continue;
        }
        assert_eq!(
            p.map.to_orig(p.map.to_prep(o as u32)),
            Some(o as u32),
            "{o} {c:?}"
        );
    }
}

fn pane(text: &str) -> crate::viewpane::ViewPane {
    let mut p = crate::viewpane::editpane_tests::doc();
    if let crate::viewpane::LoadState::Ready { loaded, .. } = &mut p.state {
        loaded.text = text.into();
    }
    p
}

/// THE bug: in a CRLF document every line above the caret dropped a byte, so a keystroke on
/// the third line landed two bytes early — inside the line before it.
#[test]
fn typing_in_a_crlf_document_lands_where_the_caret_is() {
    let mut p = pane("first line\r\n\r\nsecond para\r\n");
    p.start_editing(40);
    p.move_caret(crate::viewpane::caret::Step::Down, 40, 20);
    p.insert("X", 40, 20);
    assert_eq!(p.source_str(), Some("first line\r\n\r\nXsecond para\r\n"));
}

#[test]
fn typing_after_a_tab_lands_where_the_caret_is() {
    let mut p = pane("a\tb and c\n");
    p.start_editing(40);
    p.move_caret(crate::viewpane::caret::Step::End, 40, 20);
    p.insert("!", 40, 20);
    assert_eq!(p.source_str(), Some("a\tb and c!\n"));
}
