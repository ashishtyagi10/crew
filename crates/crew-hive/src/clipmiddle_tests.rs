use super::*;

/// What a worker's output looks like: 200 findings, and the verdict last.
fn findings_then_verdict() -> String {
    let mut s: String = (1..=200)
        .map(|n| format!("finding {n}: looked at one more thing\n"))
        .collect();
    s.push_str("VERDICT: the fix is in route.rs:412");
    s
}

/// The number the marker line says was cut.
fn cut_of(clipped: &str) -> usize {
    let from = clipped.find("\u{2026} [").expect("a marker") + "\u{2026} [".len();
    let to = clipped[from..].find(" chars cut").expect("a count") + from;
    clipped[from..to].parse().expect("a number")
}

/// The marker line with its newlines, as it sits between head and tail.
fn marker(cut: usize) -> String {
    format!("\n\u{2026} [{cut} chars cut from the middle] \u{2026}\n")
}

/// (a) The verdict at the end survives a budget far under the output, the
/// head survives with it, and the result is the budget plus one marker line.
#[test]
fn the_verdict_and_the_first_finding_both_survive() {
    let s = findings_then_verdict();
    assert!(s.chars().count() > 6_000, "fixture must bust the budget");
    let r = clip_middle(&s, 600);
    assert!(
        r.contains("VERDICT: the fix is in route.rs:412"),
        "the conclusion was cut: {r}"
    );
    assert!(r.contains("finding 1: "), "the head was cut: {r}");
    let bound = 600 + marker(cut_of(&r)).chars().count();
    assert!(
        r.chars().count() <= bound,
        "{} > {bound}",
        r.chars().count()
    );
}

/// (b) Under budget, byte-identical: no marker, no reflow, no trim.
#[test]
fn under_budget_passes_through_byte_identical() {
    let s = findings_then_verdict();
    assert_eq!(clip_middle(&s, s.chars().count()), s);
    let short = "  two 二番 lines\n\n  trailing space \n";
    assert_eq!(clip_middle(short, 100), short);
}

/// (c) Both cuts land on line boundaries: the head is whole lines from the
/// top, the tail whole lines to the bottom, and no half "finding" line
/// touches the marker on either side.
#[test]
fn both_cuts_land_on_line_boundaries() {
    let s = findings_then_verdict();
    let r = clip_middle(&s, 600);
    let cut = cut_of(&r);
    let (head, tail) = r.split_once(&marker(cut)).expect("one marker line");
    assert!(
        s.starts_with(&format!("{head}\n")),
        "head ends mid-line: {head:?}"
    );
    assert!(s.ends_with(tail), "tail is not the output's end: {tail:?}");
    let before_tail = &s[..s.len() - tail.len()];
    assert!(
        before_tail.ends_with('\n'),
        "tail starts mid-line: {tail:?}"
    );
    assert!(tail.starts_with("finding "), "{tail:?}");
    assert!(head.ends_with("looked at one more thing"), "{head:?}");
    assert_eq!(
        head.chars().count() + 1 + tail.chars().count() + cut,
        s.chars().count(),
        "the marker's count is what went"
    );
}

/// (d) One huge line has no line end to cut at: the head keeps its first
/// third of the budget and the tail the rest, by chars, and a multibyte char
/// on either side of each cut is kept whole.
#[test]
fn a_single_huge_line_is_cut_at_chars_without_splitting_one() {
    let s = "字🦀".repeat(5_000);
    assert_eq!(s.chars().count(), 10_000);
    let r = clip_middle(&s, 3_001);
    let head: String = s.chars().take(1_000).collect();
    let tail: String = s.chars().skip(10_000 - 2_001).collect();
    assert!(r.starts_with(&format!("{head}\n")), "head is 1,000 chars");
    assert!(r.ends_with(&format!("\n{tail}")), "tail is 2,001 chars");
    assert_eq!(cut_of(&r), 10_000 - 3_001);
    assert!(std::str::from_utf8(r.as_bytes()).is_ok());
}

/// A short first line then a huge one: the nearest line end would keep six
/// chars of a 1,000-char head, so the head is cut at chars instead.
#[test]
fn a_line_end_out_of_reach_falls_back_to_chars() {
    let s = format!("title\n{}\nthe end", "x".repeat(9_000));
    let r = clip_middle(&s, 3_000);
    let (head, tail) = r.split_once(&marker(cut_of(&r))).expect("marker");
    assert_eq!(head.chars().count(), 1_000, "{head:?}");
    assert!(head.starts_with("title\nxxx"), "{head:?}");
    assert!(tail.ends_with("x\nthe end"), "{tail:?}");
    assert_eq!(tail.chars().count(), 2_000);
}
