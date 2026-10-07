//! Edges of the markdown layout the screenshot surveys found.
use crate::md::render;

fn rows(text: &str, cols: usize) -> Vec<String> {
    render(text, cols)
        .iter()
        .map(|l| l.spans.iter().map(|s| s.text.as_str()).collect())
        .collect()
}

/// A blank `>` line between quoted lines keeps the bar: the gap used to cut
/// the attribution off the words it attributes.
#[test]
fn a_quote_keeps_its_bar_across_a_blank_line() {
    let r = rows(
        "> the panel sweep found it\n>\n> \u{2014} the panel sweep\n",
        40,
    );
    let quoted: Vec<&String> = r.iter().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(quoted.len(), 3, "{r:?}");
    let bar = quoted[0].chars().next().unwrap();
    assert!(quoted.iter().all(|l| l.starts_with(bar)), "{r:?}");
}

/// A child sits under its parent's TEXT: under `1. ` three columns in, not
/// two (it started under the parent's dot).
#[test]
fn a_nested_item_sits_under_its_parents_text() {
    let r = rows(
        "1. read the frame\n   1. build_frame\n   2. every band\n",
        40,
    );
    let parent = r.iter().find(|l| l.contains("read")).expect("parent");
    let child = r.iter().find(|l| l.contains("build_frame")).expect("child");
    let text_at = parent.find("read").unwrap();
    let marker_at = child.find(|c: char| !c.is_whitespace()).unwrap();
    assert_eq!(marker_at, text_at, "{parent:?} / {child:?}");
}

/// A URL or path with no space breaks after a `/` (or `?`, `=`, `.`…) in
/// the back half of the row, never in the middle of a segment.
#[test]
fn a_long_token_breaks_after_a_separator() {
    let url = "https://example.invalid/a/path/that/will/not/fit/in/one/row?with=query";
    for cols in 20..40 {
        let r = rows(url, cols);
        for row in &r[..r.len() - 1] {
            assert!(
                row.ends_with(|c: char| "/:_-.?&=".contains(c)),
                "{cols}: {r:?}"
            );
        }
        assert_eq!(r.concat(), url, "{cols}: nothing lost");
    }
}
