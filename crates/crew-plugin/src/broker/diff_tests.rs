use super::*;

#[test]
fn clean_tree_reports_a_friendly_line() {
    assert_eq!(diff_report(""), "working tree clean \u{2014} no changes");
    assert_eq!(
        diff_report("   \n  \t "),
        "working tree clean \u{2014} no changes"
    );
}

#[test]
fn small_stat_passes_through_trimmed() {
    let stat = "\n a.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n";
    assert_eq!(
        diff_report(stat),
        "a.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)"
    );
}

#[test]
fn over_cap_stat_is_truncated_with_marker() {
    let long = "x".repeat(5000);
    let out = diff_report(&long);
    assert_eq!(
        out.chars().count(),
        4000 + "\n\u{2026} (diff truncated)".chars().count()
    );
    assert!(out.starts_with(&"x".repeat(4000)), "kept the head");
    assert!(
        out.ends_with("\u{2026} (diff truncated)"),
        "marked truncated: {out}"
    );
}
