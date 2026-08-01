use super::*;

fn v(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| s.to_string()).collect()
}

#[test]
fn diff_and_patch_tags_dispatch_nothing_else_does() {
    assert!(is_diff_lang("diff"));
    assert!(is_diff_lang("patch"));
    assert!(!is_diff_lang("rust"));
    assert!(!is_diff_lang(""));
}

#[test]
fn changed_lines_take_their_own_classes() {
    assert_eq!(
        line_runs("+let x = 1;"),
        vec![("+let x = 1;".to_string(), Token::Added)]
    );
    assert_eq!(
        line_runs("-let x = 0;"),
        vec![("-let x = 0;".to_string(), Token::Removed)]
    );
    assert_eq!(
        line_runs("@@ -1,3 +1,4 @@ fn main()"),
        vec![("@@ -1,3 +1,4 @@ fn main()".to_string(), Token::Hunk)]
    );
}

#[test]
fn context_lines_stay_plain_and_empty_lines_yield_no_runs() {
    assert_eq!(
        line_runs(" unchanged context"),
        vec![(" unchanged context".to_string(), Token::Plain)]
    );
    assert!(line_runs("").is_empty());
}

/// The classic trap: `+++ b/x` and `--- a/x` are file headers, not a triple
/// addition/removal. They dim to comment ink, like `diff --git` and `index`.
#[test]
fn file_headers_are_dim_not_added_or_removed() {
    for h in [
        "+++ b/src/main.rs",
        "--- a/src/main.rs",
        "diff --git a/x b/x",
        "index 3f1a2b..9c0d4e 100644",
    ] {
        assert_eq!(line_runs(h)[0].1, Token::Comment, "{h}");
    }
}

#[test]
fn a_git_opener_alone_makes_it_a_diff() {
    assert!(looks_like_diff(&v(&["diff --git a/x b/x", "+y"])));
    assert!(looks_like_diff(&v(&["diff --git a/x b/x"])));
}

#[test]
fn a_hunk_alongside_change_lines_makes_it_a_diff() {
    assert!(looks_like_diff(&v(&["@@ -1 +1 @@", "-old", "+new"])));
    assert!(looks_like_diff(&v(&[
        "ctx",
        "@@ -2,3 +2,3 @@",
        "+only adds"
    ])));
}

/// Half the evidence is not enough: a hunk with no change lines, or change
/// markers with no hunk (a bullet list, a signature of dashes), stays plain.
#[test]
fn half_the_evidence_is_not_a_diff() {
    assert!(!looks_like_diff(&v(&["@@ -1 +1 @@", "context only"])));
    assert!(!looks_like_diff(&v(&["- a bullet", "+ a plus", "no hunk"])));
    assert!(!looks_like_diff(&v(&["fn main() {}", "let x = 1;"])));
    assert!(!looks_like_diff(&[]));
}

/// `+++`/`---` header lines must not count as the change-line evidence.
#[test]
fn header_lines_do_not_count_as_changes() {
    assert!(!looks_like_diff(&v(&["@@ -1 +1 @@", "+++ b/x", "--- a/x"])));
}
