use super::*;

/// The byte range as text, so a failure reads as the thing found rather than as two numbers.
fn url_of(src: &str, at: u32) -> Option<&str> {
    let s = link_at(src, at)?;
    Some(&src[s.url.0 as usize..s.url.1 as usize])
}

fn whole_of(src: &str, at: u32) -> Option<&str> {
    let s = link_at(src, at)?;
    Some(&src[s.whole.0 as usize..s.whole.1 as usize])
}

#[test]
fn the_url_under_the_caret_is_the_one_that_would_be_replaced() {
    let src = "see [the docs](https://example.com/a) for more";
    // Anywhere inside the link: on the text, on the marker, inside the URL itself.
    for at in [4, 8, 14, 20, 35] {
        assert_eq!(url_of(src, at), Some("https://example.com/a"), "at {at}");
    }
}

#[test]
fn a_caret_outside_a_link_is_not_in_one() {
    let src = "see [the docs](https://example.com/a) for more";
    for at in [0, 3, 37, 44] {
        assert_eq!(url_of(src, at), None, "at {at}");
    }
}

#[test]
fn the_right_link_is_found_when_a_line_holds_several() {
    let src = "[one](http://a) and [two](http://b) and [three](http://c)";
    assert_eq!(url_of(src, 2), Some("http://a"));
    assert_eq!(url_of(src, 22), Some("http://b"));
    assert_eq!(url_of(src, 42), Some("http://c"));
    assert_eq!(url_of(src, 16), None, "the word 'and' is not in a link");
}

#[test]
fn an_image_carries_its_bang() {
    // Replacing the whole span without the `!` would leave a stray bang in the file.
    let src = "![a picture](pic.png)";
    assert_eq!(whole_of(src, 5), Some("![a picture](pic.png)"));
    assert_eq!(url_of(src, 5), Some("pic.png"));
}

#[test]
fn a_url_with_brackets_in_it_survives() {
    let src = "[wiki](https://en.wikipedia.org/wiki/Foo_(bar))";
    assert_eq!(
        url_of(src, 3),
        Some("https://en.wikipedia.org/wiki/Foo_(bar)")
    );
}

#[test]
fn a_bracket_that_opens_nothing_is_not_a_link() {
    for src in [
        "a ] ( b",
        "[unclosed](http://example.com",
        "[reference][ref]",
        "text with [brackets] and (parens)",
    ] {
        assert_eq!(link_at(src, 3), None, "{src:?}");
    }
}

#[test]
fn a_link_across_a_line_break_is_not_a_link() {
    // A `]` on one line and a `[` on another is two different things that happen to be adjacent
    // in the byte stream.
    let src = "ends here]\n[starts here](http://a)";
    assert_eq!(url_of(src, 9), None);
    assert_eq!(url_of(src, 15), Some("http://a"));
}

const TABLE: &str = "| name | size |\n| --- | --- |\n| a | 10 |\n| b | 20 |\n\nafter";

#[test]
fn tab_walks_the_cells_of_a_row() {
    // "| name | size |" — from the caret on `name`, the next cell is `size`.
    let at = TABLE.find("name").unwrap() as u32;
    let next = next_cell(TABLE, at).expect("a next cell");
    assert_eq!(&TABLE[next as usize..next as usize + 4], "size");
}

#[test]
fn tab_off_the_end_of_a_row_lands_on_the_next_row_and_skips_the_divider() {
    // From `size` — the last cell of the header — the next cell is `a`, not `---`.
    let at = TABLE.find("size").unwrap() as u32;
    let next = next_cell(TABLE, at).expect("a next cell");
    assert_eq!(&TABLE[next as usize..next as usize + 1], "a");
}

#[test]
fn tab_walks_from_the_last_cell_of_one_body_row_to_the_first_of_the_next() {
    let at = TABLE.find("10").unwrap() as u32;
    let next = next_cell(TABLE, at).expect("a next cell");
    assert_eq!(&TABLE[next as usize..next as usize + 1], "b");
}

#[test]
fn tab_after_the_last_cell_of_the_last_row_has_nowhere_to_go() {
    // And "nowhere to go" is what makes the caller type spaces instead of moving the caret out
    // of the table into the paragraph below it.
    let at = TABLE.find("20").unwrap() as u32;
    assert_eq!(next_cell(TABLE, at), None);
}

#[test]
fn tab_outside_a_table_is_not_a_cell_move() {
    let at = TABLE.find("after").unwrap() as u32;
    assert_eq!(next_cell(TABLE, at), None);
    assert_eq!(next_cell("just a paragraph", 3), None);
    assert_eq!(next_cell("", 0), None);
}

#[test]
fn an_alignment_row_is_a_divider_too() {
    let src = "| a | b |\n|:--- | ---:|\n| 1 | 2 |";
    let at = src.find(" b").unwrap() as u32 + 1;
    let next = next_cell(src, at).expect("a next cell");
    assert_eq!(&src[next as usize..next as usize + 1], "1");
}
