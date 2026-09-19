use super::*;

#[test]
fn a_tag_goes_in_front_and_an_empty_meta_carries_the_tag_alone() {
    assert_eq!(tagged(SUB, "3.2s"), "sub \u{00b7} 3.2s");
    assert_eq!(tagged(SUB, ""), "sub");
}

/// `stdio` stamps the task id over whatever the construct already wrote, so
/// the tag this module looks for is rarely first.
#[test]
fn a_tag_is_found_anywhere_in_the_list() {
    assert!(has("task:7 \u{00b7} sub \u{00b7} 3.2s", SUB));
    assert!(has("sub", SUB));
    assert!(!has("task:7 \u{00b7} 3.2s", SUB));
    assert!(!has("", SUB));
}

/// A latency that merely CONTAINS the tag's letters is not the tag.
#[test]
fn a_substring_is_not_a_tag() {
    assert!(!has("subtotal \u{00b7} 3.2s", SUB));
    assert_eq!(
        strip_sub("subtotal \u{00b7} 3.2s"),
        "subtotal \u{00b7} 3.2s"
    );
}

#[test]
fn stripping_leaves_the_latency_alone() {
    assert_eq!(strip_sub("sub \u{00b7} 3.2s"), "3.2s");
    assert_eq!(strip_sub("sub"), "");
    assert_eq!(strip_sub("3.2s"), "3.2s");
    assert_eq!(strip_sub(""), "");
}
