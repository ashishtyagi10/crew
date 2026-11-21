use crate::{GridSize, HeadlessTerm, TermModel};

fn term(cols: u16, rows: u16) -> HeadlessTerm {
    HeadlessTerm::new(GridSize { cols, rows })
}

fn find(t: &HeadlessTerm, from: usize, needle: &str) -> Option<usize> {
    t.find_back(from, &mut |line: &str| line.contains(needle))
}

#[test]
fn a_match_seven_thousand_lines_back_is_found_in_one_pass() {
    let mut t = term(80, 24);
    let mut out = String::from("first error: needlezq\r\n");
    for i in 0..7000 {
        out.push_str(&format!("line {i} of filler\r\n"));
    }
    t.feed(out.as_bytes());
    // 7,001 lines above the cursor's (empty) bottom row: the needle is the
    // oldest of them, far past the 5,000 single-line steps `/find` took.
    let start = std::time::Instant::now();
    assert_eq!(find(&t, 0, "needlezq"), Some(7001));
    assert!(start.elapsed() < std::time::Duration::from_secs(1));
    // Nothing moved: the search reads the grid, it does not scroll it.
    assert_eq!(t.display_offset(), 0);
}

#[test]
fn a_word_split_by_a_soft_wrap_is_one_line() {
    let mut t = term(10, 4);
    // "abcdefghij" fills the row; "KLMN" runs onto the next.
    t.feed(b"abcdefghijKLMN\r\nnext\r\n");
    // Rows from the bottom: 0 cursor, 1 "next", 2 "KLMN", 3 "abcdefghij".
    assert_eq!(
        find(&t, 0, "ijKL"),
        Some(3),
        "joined, reported at its top row"
    );
    // Starting on the wrapped half still reads the half above it.
    assert_eq!(find(&t, 2, "ijKL"), Some(3));
    assert_eq!(find(&t, 0, "next"), Some(1));
}

#[test]
fn starting_above_a_hit_finds_the_older_one_and_then_nothing() {
    let mut t = term(20, 5);
    t.feed(b"hit one\r\nfiller\r\nhit two\r\nfiller\r\nfiller\r\nfiller\r\n");
    let newest = find(&t, 0, "hit").unwrap();
    assert_eq!(newest, 4);
    assert_eq!(find(&t, newest + 1, "hit"), Some(6));
    assert_eq!(find(&t, 7, "hit"), None);
    assert_eq!(find(&t, 10_000, "hit"), None, "past the top is a miss");
}

#[test]
fn a_wide_character_is_one_column_of_text() {
    let mut t = term(20, 3);
    t.feed("日本 ok\r\n".as_bytes());
    // Top row of three: the spacers behind 日 and 本 are not read as text.
    assert_eq!(find(&t, 0, "日本 ok"), Some(2));
}

/// The row under the view is the line the view shows last one step further
/// down, numbered one past the screen — and reading it leaves the view where
/// it was. Nothing is under the live bottom.
#[test]
fn row_below_is_the_next_line_down_and_leaves_the_view_alone() {
    let mut t = super::TermCore::new(GridSize { cols: 20, rows: 4 });
    for i in 0..30 {
        t.feed(format!("L{i}\r\n").as_bytes());
    }
    assert!(
        t.row_below(false).is_none(),
        "nothing under the live bottom"
    );
    t.scroll(5);
    let shown: String = t
        .cells(false)
        .iter()
        .filter(|c| c.row == 3)
        .map(|c| c.c)
        .collect();
    let below = t.row_below(false).unwrap();
    assert_eq!(t.display_offset(), 5);
    assert!(below.iter().all(|c| c.row == 4));
    let text: String = below.iter().map(|c| c.c).collect();
    let n: u32 = shown.trim()[1..].parse().unwrap();
    assert_eq!(text.trim(), format!("L{}", n + 1));
}
