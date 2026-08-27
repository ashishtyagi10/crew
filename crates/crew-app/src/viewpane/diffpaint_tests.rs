use super::*;

fn theme_guard() -> crate::app::ThemeGuard {
    crate::app::theme_test_guard()
}

#[test]
fn a_line_is_classified_by_the_marker_a_unified_diff_uses() {
    assert_eq!(kind_of("@@ -1,2 +1,3 @@"), Kind::Hunk);
    assert_eq!(kind_of("diff --git a/x b/x"), Kind::File);
    assert_eq!(kind_of("index 1234..5678 100644"), Kind::File);
    assert_eq!(kind_of("+added"), Kind::Added);
    assert_eq!(kind_of("-gone"), Kind::Removed);
    assert_eq!(kind_of(" same"), Kind::Context);
    assert_eq!(kind_of(""), Kind::Context);
}

/// `+++ b/file` starts with a plus and is not an added line. Getting this
/// wrong paints the file header green and pairs it with `--- a/file`.
#[test]
fn the_file_headers_are_not_a_one_line_addition_and_removal() {
    assert_eq!(kind_of("+++ b/src/main.rs"), Kind::File);
    assert_eq!(kind_of("--- a/src/main.rs"), Kind::File);
}

#[test]
fn a_single_word_change_marks_only_that_word() {
    let (old, new) = ("let total = 1;", "let total = 2;");
    let ((a0, a1), (b0, b1)) = refine(old, new).unwrap();
    assert_eq!(&old[a0..a1], "1");
    assert_eq!(&new[b0..b1], "2");
}

/// A change inside an identifier marks the identifier, not the letters that
/// happen to differ — `foo_bar` → `foo_baz` is a different name, not a `r`.
#[test]
fn a_change_inside_a_word_marks_the_whole_word() {
    let ((a0, a1), (b0, b1)) = refine("call foo_bar()", "call foo_baz()").unwrap();
    assert_eq!(&"call foo_bar()"[a0..a1], "foo_bar");
    assert_eq!(&"call foo_baz()"[b0..b1], "foo_baz");
}

#[test]
fn an_appended_tail_is_marked_and_the_shared_head_is_not() {
    let ((a0, a1), (b0, b1)) = refine("fn x() {}", "fn x() {} // note").unwrap();
    assert_eq!(a0, a1, "nothing was removed");
    assert_eq!(&"fn x() {} // note"[b0..b1], " // note");
}

/// Two lines that share almost nothing are a rewrite: marking all of both is
/// not a mark, so nothing is marked and they stay plain.
#[test]
fn a_wholesale_rewrite_is_not_refined() {
    assert_eq!(refine("alpha beta gamma", "zulu yankee xray"), None);
}

#[test]
fn identical_or_empty_lines_refine_to_nothing() {
    assert_eq!(refine("same", "same"), None);
    assert_eq!(refine("", "x"), None);
    assert_eq!(refine("x", ""), None);
}

/// The paint is lossless: exactly one entry per character of every line, so
/// the renderer can zip the two together without falling back.
#[test]
fn every_character_of_every_line_gets_exactly_one_paint() {
    let _g = theme_guard();
    let text = "diff --git a/x b/x\n@@ -1,2 +1,2 @@ fn main\n-let a = 1;\n+let a = 2;\n ok\n";
    let paints = paint(text);
    for (line, p) in text.split('\n').zip(&paints) {
        assert_eq!(line.chars().count(), p.len(), "{line:?}");
    }
}

/// The pair refines: the shared head of both lines recedes, the changed digit
/// does not, and the two are drawn in different colours.
#[test]
fn a_paired_change_dims_what_the_two_lines_share() {
    let _g = theme_guard();
    let text = "@@ -1 +1 @@\n-let a = 1;\n+let a = 2;";
    let p = paint(text);
    let removed = &p[1];
    let shared = removed[3]; // 'e' of "let"
    let changed = removed[9]; // the '1'
    assert_ne!(shared.0, changed.0, "the shared text is not dimmed");
    assert!(changed.1, "the change is not marked");
    assert!(!shared.1);
    assert_ne!(p[1][9].0, p[2][9].0, "removed and added share a colour");
}

/// Runs that do not correspond line for line are not paired at all: nothing
/// dims, because there is no honest correspondence to draw.
#[test]
fn unequal_runs_are_left_unrefined() {
    let _g = theme_guard();
    let text = "@@ -1,2 +1 @@\n-let a = 1;\n-let b = 2;\n+let a = 9;";
    let p = paint(text);
    let colours: Vec<(u8, u8, u8)> = p[1].iter().map(|(c, _)| *c).collect();
    assert!(
        colours.windows(2).all(|w| w[0] == w[1]),
        "an unpairable line was refined anyway"
    );
}

/// The hunk heading is the range; the function context after it is a note.
#[test]
fn the_hunk_heading_and_its_context_are_drawn_apart() {
    let _g = theme_guard();
    let p = paint("@@ -1,7 +1,9 @@ fn main() {");
    let head = p[0][2];
    let context = p[0][20];
    assert_ne!(head.0, context.0);
    assert!(head.1 && !context.1);
}
