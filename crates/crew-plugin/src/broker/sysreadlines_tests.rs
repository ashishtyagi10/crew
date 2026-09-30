//! `sys:read_file {"lines": N}`: a page that stops after N lines, however few
//! bytes they take.
use crate::broker::systools::{call, tools};
use serde_json::json;

/// A file of `line 1` … `line N`, and the byte each line starts at.
fn numbered(name: &str, n: usize) -> (String, Vec<usize>) {
    let rows: Vec<String> = (1..=n).map(|i| format!("line {i}\n")).collect();
    let starts = rows
        .iter()
        .scan(0, |at, r| {
            let s = *at;
            *at += r.len();
            Some(s)
        })
        .collect();
    let p = std::env::temp_dir().join(format!("crew-sysreadlines-{name}-{}", std::process::id()));
    std::fs::write(&p, rows.concat()).unwrap();
    (p.display().to_string(), starts)
}

fn read(args: serde_json::Value) -> Result<String, String> {
    call("read_file", &args.to_string())
}

#[test]
fn one_line_asked_for_is_the_page_and_the_note_goes_on_from_the_next() {
    let (path, starts) = numbered("one", 2000);
    let r = read(json!({"path": path, "line": 1777, "lines": 1})).unwrap();
    let rows: Vec<&str> = r.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(rows[0], "1777\u{2502} line 1777");
    assert_eq!(rows.len(), 2, "one row and its note: {r}");
    assert!(rows[1].contains("line 1,777 of 2,000"), "{r}");
    let next = starts[1777]; // line 1,778's first byte
    assert!(
        rows[1].ends_with(&format!("{{\"offset\": {next}}})")),
        "{r}"
    );
}

#[test]
fn lines_count_from_the_top_and_from_an_offset_alike() {
    let (path, starts) = numbered("top", 2000);
    let r = read(json!({"path": path, "lines": 3})).unwrap();
    let rows: Vec<&str> = r.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        rows[..3],
        ["1\u{2502} line 1", "2\u{2502} line 2", "3\u{2502} line 3"]
    );
    assert!(
        rows[3].ends_with(&format!("{{\"offset\": {}}})", starts[3])),
        "{r}"
    );
    let r = read(json!({"path": path, "offset": starts[3], "lines": "2"})).unwrap();
    let rows: Vec<&str> = r.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(rows[..2], ["4\u{2502} line 4", "5\u{2502} line 5"], "{r}");
    assert_eq!(rows.len(), 3);
}

#[test]
fn more_lines_than_the_file_holds_is_the_whole_file_with_no_note() {
    let (path, _) = numbered("short", 5);
    let r = read(json!({"path": path, "lines": 50})).unwrap();
    assert_eq!(r.lines().count(), 5, "{r}");
    assert!(!r.contains('\u{2026}'), "a last page has no note: {r}");
}

#[test]
fn a_count_past_the_page_still_stops_at_the_page() {
    let (path, _) = numbered("many", 2000);
    let r = read(json!({"path": path, "lines": 1500})).unwrap();
    let rows = r.lines().filter(|l| l.contains('\u{2502}')).count();
    assert!(
        rows < 1500,
        "the page's byte budget still holds: {rows} rows"
    );
    assert!(r.len() <= super::super::sysread::PAGE + 400, "{}", r.len());
}

#[test]
fn a_count_that_is_not_one_or_more_is_refused_by_name() {
    let (path, _) = numbered("bad", 10);
    for bad in [json!(0), json!(-1), json!(1.5), json!("x"), json!(true)] {
        let e = read(json!({"path": path, "lines": bad})).unwrap_err();
        assert!(e.contains("\u{201c}lines\u{201d}"), "{bad}: {e}");
    }
}

#[test]
fn the_schema_and_the_hint_both_offer_lines() {
    let t = tools().into_iter().find(|t| t.name == "read_file").unwrap();
    assert_eq!(t.input_schema["properties"]["lines"]["minimum"], 1);
    assert!(t.description.contains("\"lines\""), "{}", t.description);
}
