use super::*;
use crate::broker::systools::{call, tools};
use serde_json::json;

/// A file of `line 1` … `line N`, each ended by `eol`. 2,000 lines, not
/// 1,000: from line 412 of a 1,000-line file the rest (5,302 bytes) fits one
/// page, and a last page carries no note to check or continue from.
fn numbered(name: &str, eol: &str) -> String {
    let content: String = (1..=2000).map(|i| format!("line {i}{eol}")).collect();
    let p = std::env::temp_dir().join(format!("crew-sysreadline-{name}-{}", std::process::id()));
    std::fs::write(&p, content).unwrap();
    p.display().to_string()
}

fn read_at(path: &str, extra: serde_json::Value) -> Result<String, String> {
    let mut args = json!({ "path": path });
    args.as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    call("read_file", &args.to_string())
}

/// The page, and the note after it: `(lines A–B of N, …{"offset": X})`.
fn split(r: &str) -> (&str, &str) {
    r.rsplit_once('\n').expect("a page with a note")
}

/// A page's first line and its note: enough to tell two pages apart without
/// printing both whole when they differ.
fn ends(r: &str) -> (&str, &str) {
    let first = r.lines().next().unwrap_or("");
    (first, r.rsplit('\n').next().unwrap_or(""))
}

fn next_offset(note: &str) -> usize {
    let json = &note[note.rfind('{').unwrap()..note.len() - 1];
    serde_json::from_str::<serde_json::Value>(json).unwrap()["offset"]
        .as_u64()
        .unwrap() as usize
}

#[test]
fn a_page_asked_for_by_line_starts_exactly_at_that_line() {
    let path = numbered("at", "\n");
    let r = read_at(&path, json!({ "line": 412 })).unwrap();
    let (page, note) = split(&r);
    assert_eq!(page.lines().next(), Some("line 412"), "{note}");
    assert!(note.starts_with("\u{2026} (lines 412\u{2013}"), "{note}");
    assert!(note.contains(" of 2,000, "), "{note}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_notes_offset_goes_on_from_the_line_page_without_a_gap() {
    let path = numbered("next", "\n");
    let first = read_at(&path, json!({ "line": 412 })).unwrap();
    let (page, note) = split(&first);
    let last: usize = page.lines().last().unwrap()["line ".len()..]
        .parse()
        .unwrap();
    assert!(
        note.contains(&format!("412\u{2013}{} of", grouped(last))),
        "{note}"
    );
    let second = read_at(&path, json!({ "offset": next_offset(note) })).unwrap();
    assert_eq!(
        second.lines().next(),
        Some(format!("line {}", last + 1).as_str())
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn line_takes_a_quoted_number_and_refuses_zero_and_negatives() {
    let path = numbered("args", "\n");
    let quoted = read_at(&path, json!({ "line": "412" })).unwrap();
    let number = read_at(&path, json!({ "line": 412 })).unwrap();
    assert_eq!(ends(&quoted), ends(&number));
    for bad in [
        json!(0),
        json!(-3),
        json!("0"),
        json!("-3"),
        json!(4.5),
        json!(true),
    ] {
        let e = read_at(&path, json!({ "line": bad })).unwrap_err();
        assert!(e.starts_with("invalid \u{201c}line\u{201d}"), "{bad}: {e}");
    }
    let first = read_at(&path, json!({ "line": 1 })).unwrap();
    let top = read_at(&path, json!({})).unwrap();
    assert_eq!(ends(&first), ends(&top), "line 1 is the top");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_line_past_the_end_names_how_many_lines_there_are() {
    let path = numbered("past", "\n");
    assert_eq!(
        read_at(&path, json!({ "line": 2000 })).unwrap(),
        "line 2000\n"
    );
    let r = read_at(&path, json!({ "line": 2001 })).unwrap();
    assert_eq!(
        r,
        "\u{2026} (line 2,001 is past the end \u{2014} the file has 2,000 lines)"
    );
    // An unterminated last line counts; an empty file has none.
    std::fs::write(&path, "a\nb").unwrap();
    assert_eq!(read_at(&path, json!({ "line": 2 })).unwrap(), "b");
    let r = read_at(&path, json!({ "line": 3 })).unwrap();
    assert!(r.ends_with("the file has 2 lines)"), "{r}");
    std::fs::write(&path, "").unwrap();
    let r = read_at(&path, json!({ "line": 1 })).unwrap();
    assert!(r.ends_with("the file has 0 lines)"), "{r}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn line_and_offset_together_is_one_line_saying_pass_one() {
    let path = numbered("both", "\n");
    let e = read_at(&path, json!({ "line": 412, "offset": 0 })).unwrap_err();
    assert!(!e.contains('\n'), "{e}");
    assert!(e.contains("not both"), "{e}");
    // A null offset is no offset.
    assert!(read_at(&path, json!({ "line": 412, "offset": null })).is_ok());
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_crlf_file_numbers_its_lines_the_same() {
    let path = numbered("crlf", "\r\n");
    let r = read_at(&path, json!({ "line": 412 })).unwrap();
    let (page, note) = split(&r);
    assert_eq!(page.lines().next(), Some("line 412"), "{note}");
    assert!(note.starts_with("\u{2026} (lines 412\u{2013}"), "{note}");
    let r = read_at(&path, json!({ "line": 2001 })).unwrap();
    assert!(r.ends_with("the file has 2,000 lines)"), "{r}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_huge_file_seeks_near_lines_and_refuses_far_ones() {
    let content: String = (0..LINES_UP_TO / 16 + 2)
        .map(|i| format!("{i:015}\n"))
        .collect();
    let path = std::env::temp_dir().join(format!("crew-sysreadline-huge-{}", std::process::id()));
    std::fs::write(&path, content).unwrap();
    let path = path.display().to_string();
    let r = read_at(&path, json!({ "line": 10 })).unwrap();
    assert!(r.starts_with("000000000000009\n"), "{}", &r[..40]);
    // The last line starts one byte past the scan's cap: there, but too far.
    let e = read_at(&path, json!({ "line": LINES_UP_TO / 16 + 2 })).unwrap_err();
    assert!(e.contains("too big to seek by line"), "{e}");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn the_text_hint_shows_the_line_example_whole() {
    // Text-mode providers see one clipped line per tool; the example is what
    // teaches the argument, so it must survive the clip.
    let hint = crate::broker::toolcall::hint_for(&tools());
    assert!(
        hint.contains("{\"path\": \"a.rs\", \"line\": 412}"),
        "{hint}"
    );
}
