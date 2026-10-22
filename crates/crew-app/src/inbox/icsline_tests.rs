use super::{props, text};

#[test]
fn a_folded_line_is_read_whole_and_a_quoted_parameter_keeps_its_colons() {
    let ics = "SUMMARY:Q4 plan\r\n ning review\r\nORGANIZER;CN=\"Lee; Ann: PM\";ROLE=CHAIR:mailto:ann@x.org\r\nDESCRIPTION:one\r\n\ttwo\r\n";
    let p = props(ics);
    assert_eq!(
        (p[0].name.as_str(), p[0].value.as_str()),
        ("SUMMARY", "Q4 planning review")
    );
    assert_eq!(p[1].param("cn"), Some("Lee; Ann: PM"));
    assert_eq!(p[1].param("ROLE"), Some("CHAIR"));
    assert_eq!(p[1].value, "mailto:ann@x.org");
    assert_eq!(p[2].value, "onetwo");
}

#[test]
fn a_text_value_has_its_escapes_undone() {
    assert_eq!(
        text(r"Room 4\, floor 2\; east\nbring a laptop \\o/"),
        "Room 4, floor 2; east\nbring a laptop \\o/"
    );
}

#[test]
fn a_line_with_no_colon_is_passed_over() {
    assert_eq!(props("garbage\r\nUID:a\r\n").len(), 1);
}
