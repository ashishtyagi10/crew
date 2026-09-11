use super::{array_text, without_trailing_commas};

const PLAN: &str = r#"[{"id":0,"title":"t","prompt":"p","deps":[]}]"#;

#[test]
fn a_bare_array_is_returned_whole() {
    assert_eq!(array_text(PLAN), Some(PLAN));
    assert_eq!(array_text("  \n[]\n"), Some("[]"));
}

#[test]
fn a_json_fence_is_stripped() {
    let fenced = format!("```json\n{PLAN}\n```");
    assert_eq!(array_text(&fenced), Some(PLAN));
    let bare_fence = format!("```\n{PLAN}\n```");
    assert_eq!(array_text(&bare_fence), Some(PLAN));
}

#[test]
fn prose_before_and_after_is_dropped_even_when_it_has_brackets() {
    let reply = format!("Here is the plan:\n{PLAN}\nSee [docs] for the shape [here].");
    assert_eq!(array_text(&reply), Some(PLAN));
}

#[test]
fn brackets_inside_strings_do_not_close_the_array() {
    let plan = r#"[{"id":0,"title":"use arr[0] and }","prompt":"say \"[hi]\"","deps":[]}]"#;
    let reply = format!("{plan} — done]");
    assert_eq!(array_text(&reply), Some(plan));
}

#[test]
fn no_array_or_an_unclosed_one_is_none() {
    assert_eq!(array_text("I cannot plan that."), None);
    assert_eq!(array_text(""), None);
    assert_eq!(
        array_text(r#"[{"id":0,"title":"cut off by max_tokens"#),
        None,
        "a truncated array must fail, never be closed for the model"
    );
}

#[test]
fn a_trailing_comma_before_a_closing_bracket_is_dropped() {
    let sloppy = r#"[{"id":0,"deps":[0,],},]"#;
    assert_eq!(without_trailing_commas(sloppy), r#"[{"id":0,"deps":[0]}]"#);
    let spaced = "[1, 2,\n  ]";
    assert_eq!(without_trailing_commas(spaced), "[1, 2\n  ]");
}

#[test]
fn a_comma_inside_a_string_and_valid_json_are_untouched() {
    let s = r#"["a, ]", "b,}"]"#;
    assert_eq!(without_trailing_commas(s), s);
    assert_eq!(without_trailing_commas(PLAN), PLAN);
}
