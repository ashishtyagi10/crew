use super::*;

#[test]
fn the_first_token_of_each_sigil_leaves_the_title() {
    assert_eq!(
        extract_tags("ship notes @crew #priya"),
        (
            "ship notes".to_string(),
            Some("crew".to_string()),
            Some("priya".to_string())
        )
    );
}

#[test]
fn a_second_tag_on_an_axis_stays_title_text() {
    // One project and one owner per item; `re-run #966` is prose, not a
    // second assignee, so it has to survive into the title.
    assert_eq!(
        extract_tags("re-run #966 #priya @crew @extra"),
        (
            "re-run #966 @extra".to_string(),
            Some("crew".to_string()),
            Some("priya".to_string())
        )
    );
}

#[test]
fn an_all_digit_hash_is_an_issue_number_not_a_person() {
    assert!(!names_a_person("966"));
    assert!(names_a_person("priya"));
    assert!(names_a_person("v2"));
    let chars: Vec<char> = "re-run #966 #priya".chars().collect();
    assert_eq!(
        tag_spans(&chars),
        vec![(12, 18, '#')],
        "the live tint must not colour a ticket number either"
    );
}

#[test]
fn a_bare_sigil_is_not_a_tag() {
    assert_eq!(
        extract_tags("weird # thing @"),
        ("weird # thing @".to_string(), None, None)
    );
}

#[test]
fn tag_spans_carry_their_sigil_and_skip_bare_ones() {
    let chars: Vec<char> = "pay @home # rent #me".chars().collect();
    assert_eq!(
        tag_spans(&chars),
        vec![(4, 9, '@'), (17, 20, '#')],
        "the lone `#` at 10 is one char long, so it is not a span"
    );
}

#[test]
fn lone_tag_reads_the_axis_and_an_empty_name_clears_it() {
    assert_eq!(lone_tag("#priya"), Some(('#', "priya")));
    assert_eq!(lone_tag("@crew"), Some(('@', "crew")));
    assert_eq!(lone_tag("#"), Some(('#', "")), "a bare sigil clears");
    assert_eq!(lone_tag("@crew now"), None, "not alone: that's an item");
    assert_eq!(lone_tag("pay rent"), None);
}
