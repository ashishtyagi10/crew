use super::{address, first_bad, header, names, split, valid, without};

#[test]
fn a_comma_in_a_quoted_name_does_not_split_it() {
    assert_eq!(
        split("\"Lee, Ann\" <ann@x.org>, bo@y.org; ,  cy@z.org "),
        ["\"Lee, Ann\" <ann@x.org>", "bo@y.org", "cy@z.org"]
    );
    assert!(split(" , ").is_empty());
    assert_eq!(address("\"Lee, Ann\" <ann@x.org>"), "ann@x.org");
    assert_eq!(address(" bo@y.org "), "bo@y.org");
}

#[test]
fn only_something_at_a_dotted_domain_is_an_address() {
    for good in ["a@x.org", "Ann <a.b+c@mail.x.org>"] {
        assert!(valid(good), "{good}");
    }
    for bad in [
        "bob",
        "a@",
        "@x.org",
        "a@x",
        "a b@x.org",
        "a@x.org.",
        "a@@x.org",
    ] {
        assert!(!valid(bad), "{bad}");
    }
    assert_eq!(first_bad("a@x.org, bob, c"), Some("bob".into()));
    assert_eq!(first_bad(""), None);
}

#[test]
fn me_and_a_second_copy_drop_out_in_order() {
    let all: Vec<String> = ["Ann <ann@x.org>", "ME@y.org", "bo@y.org", "ANN@x.org"]
        .map(String::from)
        .into();
    assert_eq!(
        without(&all, &["me@y.org"]),
        ["Ann <ann@x.org>", "bo@y.org"]
    );
}

#[test]
fn a_name_outside_ascii_is_encoded_and_the_address_kept() {
    assert_eq!(header("ann@x.org,bo@y.org"), "ann@x.org, bo@y.org");
    let h = header("\"Zoë\" <zoe@x.org>");
    assert!(
        h.starts_with("=?UTF-8?B?") && h.ends_with("?= <zoe@x.org>"),
        "{h}"
    );
    assert!(h.is_ascii());
}

#[test]
fn a_note_names_one_two_or_the_first_and_a_count() {
    assert_eq!(names("Ann Lee <ann@x.org>"), "Ann Lee");
    assert_eq!(names("Ann <a@x.org>, bo@y.org"), "Ann and bo@y.org");
    assert_eq!(names("a@x.org, b@x.org, c@x.org"), "a@x.org and 2 others");
}
