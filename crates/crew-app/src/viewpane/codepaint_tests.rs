use super::*;

#[test]
fn a_keyword_takes_the_theme_keyword_slot_and_bold() {
    // "let" is a Rust keyword; its first char is index 0.
    let paint = line_paint("let x = 1;", "rust", (10, 10, 10));
    let (fg, bold) = paint[0];
    assert_eq!(
        fg,
        crate::chatink::token_fg(Token::Keyword),
        "a keyword's colour should come from chatink's derived slot"
    );
    assert!(bold, "chatmd marks keywords by weight, not a fourth colour");
}

#[test]
fn a_plain_identifier_keeps_the_callers_ink_unbold() {
    // "x" in "let x = 1;" is the identifier at index 4.
    let ink = (10, 10, 10);
    let paint = line_paint("let x = 1;", "rust", ink);
    let (fg, bold) = paint[4];
    assert_eq!(fg, ink);
    assert!(!bold);
    // The regression this whole fix exists for: a keyword indistinguishable
    // from a plain identifier delivers no colouring at all.
    assert_ne!(
        paint[0], paint[4],
        "keyword cell must differ from plain cell"
    );
}

#[test]
fn a_comment_takes_the_theme_comment_slot() {
    let paint = line_paint("x // trailing note", "rust", (10, 10, 10));
    // The "/" that opens "// trailing note" is at index 2.
    let (fg, _bold) = paint[2];
    assert_eq!(fg, crate::chatink::token_fg(Token::Comment));
}

#[test]
fn an_empty_lang_paints_every_char_the_callers_ink_unbold() {
    let ink = (5, 6, 7);
    let paint = line_paint("fn a() {}", "", ink);
    assert!(
        paint.iter().all(|&(fg, bold)| fg == ink && !bold),
        "no lexer means no colouring at all — the pre-Fix-1 behaviour"
    );
}

#[test]
fn every_character_of_the_line_is_covered_exactly_once() {
    let line = "// a comment with \"a string\" and a keyword fn";
    let paint = line_paint(line, "rust", (1, 2, 3));
    assert_eq!(paint.len(), line.chars().count());
}
