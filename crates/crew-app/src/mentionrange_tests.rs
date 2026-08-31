use super::*;

#[test]
fn a_span_and_a_single_line_both_parse() {
    assert_eq!(
        split("src/main.rs:120-180"),
        ("src/main.rs", Some((120, 180)))
    );
    assert_eq!(split("src/main.rs:120"), ("src/main.rs", Some((120, 120))));
}

#[test]
fn a_plain_path_has_no_range() {
    assert_eq!(split("src/main.rs"), ("src/main.rs", None));
    assert_eq!(split("README"), ("README", None));
}

/// A colon is a legal character in a filename. Truncating a name to the
/// part before it would lose the file entirely, which is worse than
/// having no ranges.
#[test]
fn a_colon_that_is_not_a_range_stays_in_the_name() {
    assert_eq!(split("notes:draft.md"), ("notes:draft.md", None));
    assert_eq!(split("a:b:c"), ("a:b:c", None));
    assert_eq!(split(":40"), (":40", None), "no path in front of it");
    assert_eq!(split("x:"), ("x:", None), "no numbers behind it");
}

/// `skill:` mentions are handled before this is ever reached, but a name
/// that merely looks like one must not acquire a range either.
#[test]
fn a_word_suffix_is_never_a_range() {
    assert_eq!(split("skill:review"), ("skill:review", None));
    assert_eq!(split("host:8080x"), ("host:8080x", None));
}

#[test]
fn nonsense_ranges_are_not_ranges() {
    assert_eq!(split("f.rs:0"), ("f.rs:0", None), "there is no line 0");
    assert_eq!(split("f.rs:40-10"), ("f.rs:40-10", None), "backwards");
    assert_eq!(split("f.rs:-5"), ("f.rs:-5", None));
}

#[test]
fn slicing_is_one_based_and_inclusive() {
    let text = "one\ntwo\nthree\nfour\nfive";
    assert_eq!(slice(text, (2, 4)).unwrap(), "two\nthree\nfour");
    assert_eq!(slice(text, (1, 1)).unwrap(), "one");
    assert_eq!(slice(text, (5, 5)).unwrap(), "five");
}

/// Asking past the end is a reasonable way to say "to the end"; asking
/// for a start past the end selects nothing, and must say so rather than
/// silently attaching an empty block.
#[test]
fn an_end_past_the_file_stops_there_and_a_start_past_it_is_nothing() {
    let text = "one\ntwo\nthree";
    assert_eq!(slice(text, (2, 900)).unwrap(), "two\nthree");
    assert_eq!(slice(text, (4, 9)), None);
    assert_eq!(slice("", (1, 5)), None);
}

#[test]
fn the_label_reads_as_one_line_or_several() {
    assert_eq!(label((120, 180)), "lines 120-180");
    assert_eq!(label((7, 7)), "line 7");
}
