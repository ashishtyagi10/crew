use super::super::{grouped, read_file, PAGE};
use super::*;
use crate::broker::editrows::carries_numbers;

fn temp(name: &str, content: &str) -> String {
    let p = std::env::temp_dir().join(format!("crew-sysreadrows-{name}-{}", std::process::id()));
    std::fs::write(&p, content).unwrap();
    p.display().to_string()
}

/// A row's number and its text: `  41│ text` → (41, "text").
fn row(r: &str) -> (usize, &str) {
    let (n, text) = r.split_once("\u{2502} ").expect("a numbered row");
    (n.trim_start().parse().expect("a line number"), text)
}

#[test]
fn rows_are_numbered_the_way_an_edit_shows_them() {
    assert_eq!(number("a\nb\n", 9), " 9\u{2502} a\n10\u{2502} b\n");
    assert_eq!(number("x", 1), "1\u{2502} x");
    assert_eq!(
        number("a\n\nc", 99),
        " 99\u{2502} a\n100\u{2502} \n101\u{2502} c"
    );
    assert_eq!(number("", 7), "");
}

#[test]
fn taking_the_numbers_off_gives_back_the_file_byte_for_byte() {
    let text = "fn a() {\r\n\tlet \u{e9} = 1;\r\n}\r\n\nlast";
    let back: String = number(text, 998)
        .split_inclusive('\n')
        .map(|r| row(r).1)
        .collect();
    assert_eq!(back, text);
}

#[test]
fn a_page_of_short_lines_shows_fewer_bytes_so_the_numbered_rows_fit() {
    let text: String = (1..=2_000).map(|i| format!("{i}\n")).collect();
    let n = fit(&text, 1, PAGE);
    assert!(n < PAGE && text[..n].ends_with('\n'), "{n}");
    let rows = number(&text[..n], 1);
    assert!(rows.len() <= PAGE, "{} bytes", rows.len());
    // One more line would not have fitted.
    let one_more = text[n..].find('\n').unwrap() + 1;
    assert!(number(&text[..n + one_more], 1).len() > PAGE);
    // What fits whole comes back whole.
    assert_eq!(fit("a\nb", 1, PAGE), 3);
}

#[test]
fn a_line_longer_than_the_page_is_cut_inside_at_a_character() {
    let text = "\u{65e5}".repeat(3_000); // 9,000 bytes, three a char, no newline
    let n = fit(&text, 41, PAGE);
    assert!(text.is_char_boundary(n) && n > PAGE - 9, "{n}");
    assert!(number(&text[..n], 41).len() <= PAGE);
}

/// The live probe: 3,000 look-alike numbers, the agent asked for line 1,777
/// and read off the number on line 2,333, because nothing on the page said
/// which row was which. Now the row it wants carries its number.
#[test]
fn the_row_a_line_read_asks_for_is_the_row_that_says_so() {
    let values: Vec<u64> = (1..=3_001u64)
        .map(|i| i.wrapping_mul(2_654_435_761) % 100_000)
        .collect();
    let file: String = values.iter().map(|v| format!("{v}\n")).collect();
    let path = temp("probe", &file);
    let line = serde_json::json!({"line": 1_777});
    let r = crate::broker::sysreadline::read(&path, &line).unwrap();
    let (page, note) = r.rsplit_once('\n').unwrap();
    let want = values[1_776].to_string();
    assert_eq!(row(page.lines().next().unwrap()), (1_777, want.as_str()));
    for r in page.lines() {
        let (n, text) = row(r);
        assert_eq!(text, values[n - 1].to_string(), "row {n}");
    }
    let last = row(page.lines().last().unwrap()).0;
    let lines = format!("\u{2026} (lines 1,777\u{2013}{} of 3,001, ", grouped(last));
    assert!(note.starts_with(&lines), "{note}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_row_copied_from_a_read_page_into_an_edit_is_caught() {
    let file: String = (1..=300)
        .map(|i| format!("    let x{i} = {i};\n"))
        .collect();
    let path = temp("copied", &file);
    let r = read_file(&path, 0).unwrap();
    let rows: Vec<&str> = r.lines().skip(40).take(2).collect();
    assert_eq!(rows[0], " 41\u{2502}     let x41 = 41;");
    let old = rows.join("\n");
    assert!(carries_numbers(&old), "{old}");
    assert!(carries_numbers(rows[1]), "{}", rows[1]);
    let e = crate::broker::sysedit::replace(&file, &old, "").unwrap_err();
    assert!(e.contains("line numbers"), "{e}");
    let _ = std::fs::remove_file(&path);
}
