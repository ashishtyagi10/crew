//! Where the chat's wrap breaks a row.
use super::wrap_indices;

fn rows(s: &str, cols: usize) -> Vec<String> {
    let full: Vec<char> = s.chars().collect();
    wrap_indices(&full, cols)
        .into_iter()
        .map(|(a, b)| full[a..b].iter().collect())
        .collect()
}

/// A list's separator ends the row, as a comma does — the next row never
/// opens on it (`· 12s`), which is the prose wrap's rule too. From a width
/// that holds the longest item and its dot: narrower, a lone word fills the
/// row and there is no other place to break.
#[test]
fn no_row_opens_on_a_separator() {
    let s = "3 done \u{b7} 1 failed \u{b7} 12s total \u{b7} telegram:8812 \u{b7} slack:9000";
    for cols in 16..40 {
        for r in rows(s, cols).iter().skip(1) {
            assert!(
                !r.starts_with('\u{b7}'),
                "{cols}: a row opens on the dot: {r:?}"
            );
        }
    }
    let s = "the plan is ready \u{2014} or draft another";
    for cols in 8..30 {
        for r in rows(s, cols).iter().skip(1) {
            assert!(
                !r.starts_with('\u{2014}'),
                "{cols}: a row opens on the dash: {r:?}"
            );
        }
    }
}

/// Every row still fits: the moved break never overruns the width.
#[test]
fn a_moved_break_still_fits() {
    let s = "a \u{b7} b \u{b7} c \u{b7} d \u{b7} e \u{b7} f";
    for cols in 3..14 {
        for r in rows(s, cols) {
            assert!(r.chars().count() <= cols, "{cols}: {r:?}");
        }
    }
}
