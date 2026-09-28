use super::*;

fn lines(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn parameters_are_cut_to_their_names() {
    assert_eq!(
        compact(
            "pub fn frame(env: &Envelope, intro: Option<&str>, peers: &[String]) -> String",
            4
        ),
        "pub fn frame(env, intro, peers) -> String"
    );
}

/// `pub(crate)`'s parentheses come before the name and a closure bound's
/// come inside the generics; neither is the parameter list.
#[test]
fn the_parameter_list_is_the_first_after_the_name_and_its_generics() {
    let sig = "pub(crate) fn map<F: Fn(u8) -> u8>(f: F, xs: Vec<u8>) -> Vec<u8>";
    assert_eq!(
        compact(sig, 11),
        "pub(crate) fn map<F: Fn(u8) -> u8>(f, xs) -> Vec<u8>"
    );
}

#[test]
fn self_patterns_defaults_and_stars_keep_what_names_them() {
    assert_eq!(
        compact(
            "fn a(&mut self, mut n: usize, (x, y): (u8, u8), p: Point<'_>)",
            0
        ),
        "fn a(&mut self, n, (x, y), p)"
    );
    assert_eq!(
        compact(
            "def f(self, *args, key: Dict[str, int] = {}, cb=lambda x: x, **kw) -> None",
            0
        ),
        "def f(self, *args, key, cb, **kw) -> None"
    );
}

#[test]
fn a_head_runs_over_lines_while_its_brackets_are_open_and_stops_at_a_stop() {
    let src = lines(&[
        "pub fn frame(",
        "    env: &Envelope,",
        "    peers: &[String],",
        ") -> String {",
        "    body()",
    ]);
    let h = head(&src, 4, &["{", ";"]);
    assert_eq!(
        h,
        "pub fn frame( env: &Envelope, peers: &[String], ) -> String"
    );
    assert_eq!(compact(&h, 4), "pub fn frame(env, peers) -> String");
    // A stop inside brackets, or before `from`, is not one.
    assert_eq!(
        head(&lines(&["pub(crate) struct Id([u8; 4]);"]), 11, &["(", ";"]),
        "pub(crate) struct Id"
    );
}

#[test]
fn a_long_row_is_clipped_to_a_hundred_chars() {
    let long = format!("fn f() -> {}", "Vec<".repeat(40));
    let row = compact(&long, 0);
    assert_eq!(row.chars().count(), ROW_CHARS + 1, "{row}");
    assert!(row.ends_with('\u{2026}'), "{row}");
}
