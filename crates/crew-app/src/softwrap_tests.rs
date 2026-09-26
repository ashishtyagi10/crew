//! Wrapped code breaks where an editor would, and loses nothing.
use crate::chatwidth::{fit_end, soft_end};

fn rows(line: &str, w: usize) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut s = 0;
    while s < chars.len() {
        let e = soft_end(&chars, s, fit_end(&chars, s, w));
        out.push(chars[s..e].iter().collect());
        s = e;
    }
    out
}

#[test]
fn a_row_breaks_after_a_space_not_inside_a_token() {
    let line = "fn arc(p: (f32, f32), r: f32, half_w: f32, a0: f32, a1: f32) -> f32 {";
    for w in 20..60 {
        let r = rows(line, w);
        assert_eq!(r.concat(), line, "{w}: the rows are the line, verbatim");
        for pair in r.windows(2) {
            let (a, b) = (
                pair[0].chars().last().unwrap(),
                pair[1].chars().next().unwrap(),
            );
            assert!(
                a.is_whitespace()
                    || ",;(".contains(a)
                    || !b.is_alphanumeric()
                    || !a.is_alphanumeric(),
                "{w}: `{}` | `{}` splits a token",
                pair[0],
                pair[1]
            );
            assert!(
                !pair[1].starts_with(' '),
                "{w}: a row opens on a space: {r:?}"
            );
        }
    }
}

#[test]
fn a_long_token_still_hard_cuts() {
    let line = "x".repeat(50);
    assert_eq!(
        rows(&line, 20),
        ["x".repeat(20), "x".repeat(20), "x".repeat(10)]
    );
}

#[test]
fn the_last_row_and_a_near_start_space_are_left_alone() {
    let chars: Vec<char> = "a bcdefghijklmnop".chars().collect();
    assert_eq!(
        soft_end(&chars, 0, 10),
        10,
        "a space in the front half is too early"
    );
    assert_eq!(soft_end(&chars, 0, chars.len()), chars.len());
}
