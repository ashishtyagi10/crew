use super::*;

#[test]
fn text_at_or_under_the_cap_is_returned_unchanged() {
    let body = vec!["x"; MAX_RENDER_LINES].join("\n");
    let (text, real) = cap_render_lines(&body);
    assert_eq!(text, body);
    assert_eq!(real, None, "a file at exactly the cap is not truncated");
}

#[test]
fn text_over_the_cap_is_cut_to_exactly_the_cap() {
    let total = MAX_RENDER_LINES + 500;
    let body = vec!["x"; total].join("\n");
    let (text, real) = cap_render_lines(&body);
    assert_eq!(
        text.split('\n').count(),
        MAX_RENDER_LINES,
        "capped text must contain exactly the cap's worth of lines, not merely no more than it"
    );
    assert_eq!(real, Some(total), "reports the real, uncapped count");
}

#[test]
fn the_cut_lands_on_a_newline_boundary_not_mid_line() {
    // A cut that landed inside a line rather than on the '\n' before it
    // would silently glue two source lines into one in the rendered output.
    let total = MAX_RENDER_LINES + 1;
    let body = vec!["abc"; total].join("\n");
    let (text, _) = cap_render_lines(&body);
    assert!(
        text.split('\n').all(|l| l == "abc"),
        "every capped line must be a whole source line: {text:?}"
    );
}
