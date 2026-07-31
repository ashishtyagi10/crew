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

/// Part 2's guard: even with a real lexer for `lang`, a line longer than
/// `MAX_COLOURED_LINE_BYTES` bails to uniform `ink` rather than tokenizing —
/// belt and braces against Part 1's fix, so a keyword that would normally
/// take the theme's keyword slot must NOT stand out here.
#[test]
fn a_line_over_the_length_cap_paints_uniformly_even_with_a_keyword() {
    let filler = "x".repeat(MAX_COLOURED_LINE_BYTES);
    let line = format!("let {filler} = 1;");
    assert!(line.len() > MAX_COLOURED_LINE_BYTES);
    let ink = (9, 9, 9);
    let paint = line_paint(&line, "rust", ink);
    assert_eq!(paint.len(), line.chars().count());
    assert!(
        paint.iter().all(|&(fg, bold)| fg == ink && !bold),
        "a line over the cap must bail to uniform ink rather than be tokenized"
    );
}

/// The companion to the cap test above: a line UNDER the cap must still be
/// tokenized normally. Without this, a mutation that always bails to uniform
/// ink (cap of zero, in effect) would pass the over-the-cap test too.
#[test]
fn a_line_at_or_under_the_cap_is_still_tokenized() {
    let base = "let x = 1; // ";
    let fill = "a".repeat(MAX_COLOURED_LINE_BYTES - base.len() - 1);
    let line = format!("{base}{fill}");
    assert!(line.len() < MAX_COLOURED_LINE_BYTES);
    let ink = (9, 9, 9);
    let paint = line_paint(&line, "rust", ink);
    assert_ne!(
        paint[0].0, ink,
        "\"let\" is a keyword and must still be coloured under the cap"
    );
}
