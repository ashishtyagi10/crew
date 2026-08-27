use super::*;

const DIFF: &str = "diff --git a/src/main.rs b/src/main.rs\nindex 1..2 100644\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,3 +1,3 @@ fn main() {\n-let a = 1;\n+let a = 2;\n@@ -20,2 +20,2 @@\n ok\ndiff --git a/README.md b/README.md\n@@ -1 +1 @@ heading\n-x\n+y\n";

#[test]
fn every_file_and_every_hunk_is_a_landmark_in_order() {
    let marks = diff_marks(DIFF);
    let labels: Vec<&str> = marks.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(
        labels,
        vec![
            "src/main.rs",
            "fn main() {",
            "@@ -20,2 +20,2 @@",
            "README.md",
            "heading",
        ]
    );
    let rows: Vec<usize> = marks.iter().map(|(i, _)| *i).collect();
    assert!(rows.windows(2).all(|w| w[0] < w[1]), "{rows:?}");
}

/// A hunk with no function context is still a landmark — named by its range,
/// since a blank row in the outline is worse than a technical one.
#[test]
fn a_hunk_without_context_is_named_by_its_range() {
    let marks = diff_marks("@@ -20,2 +20,2 @@\n");
    assert_eq!(marks, vec![(0, "@@ -20,2 +20,2 @@".to_string())]);
}

/// A rename lists under the name the file has now, not the one it had.
#[test]
fn a_renamed_file_is_listed_under_its_new_name() {
    let marks = diff_marks("diff --git a/old/path.rs b/new/path.rs\n");
    assert_eq!(marks[0].1, "new/path.rs");
}

/// Ordinary text has no structure to step through — `]` must not invent any.
#[test]
fn a_file_that_is_not_a_diff_has_no_landmarks() {
    assert!(diff_marks("hello\nworld\n").is_empty());
    assert!(diff_marks("").is_empty());
    // A line that merely mentions a hunk marker is not one.
    assert!(diff_marks("see the @@ marker").is_empty());
}

fn marks(rows: &[usize]) -> Vec<Mark> {
    rows.iter()
        .map(|&row| Mark {
            row,
            label: format!("m{row}"),
        })
        .collect()
}

#[test]
fn stepping_down_lands_on_the_next_landmark_below_the_view() {
    let m = marks(&[0, 10, 25]);
    assert_eq!(step(&m, 0, true).map(|m| m.row), Some(10));
    assert_eq!(step(&m, 9, true).map(|m| m.row), Some(10));
    assert_eq!(step(&m, 10, true).map(|m| m.row), Some(25));
}

#[test]
fn stepping_up_lands_on_the_landmark_above_the_view() {
    let m = marks(&[0, 10, 25]);
    assert_eq!(step(&m, 25, false).map(|m| m.row), Some(10));
    assert_eq!(step(&m, 11, false).map(|m| m.row), Some(10));
    assert_eq!(step(&m, 10, false).map(|m| m.row), Some(0));
}

/// At either end there is nowhere to go, and the view stays put: a review has
/// an end, and wrapping to the top from it is how you lose your place.
#[test]
fn the_ends_do_not_wrap() {
    let m = marks(&[0, 10]);
    assert_eq!(step(&m, 10, true), None);
    assert_eq!(step(&m, 0, false), None);
    assert_eq!(step(&[], 0, true), None);
    assert_eq!(step(&[], 0, false), None);
}
