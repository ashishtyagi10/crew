use super::*;

#[test]
fn claude_codes_todowrite_reads_as_steps() {
    let args = r#"{"todos":[
        {"content":"Read the failing test","status":"completed","activeForm":"Reading"},
        {"content":"Find the root cause","status":"in_progress","activeForm":"Finding"},
        {"content":"Fix and re-run","status":"pending","activeForm":"Fixing"}]}"#;
    let s = steps(args).unwrap();
    let states: Vec<StepState> = s.iter().map(|s| s.state).collect();
    assert_eq!(
        states,
        [StepState::Done, StepState::Active, StepState::Pending]
    );
    assert_eq!(s[1].text, "Find the root cause");
}

#[test]
fn crews_own_list_reads_the_same_way() {
    let args = r#"{"items":[{"text":"write   the\ndocs","status":"done"},{"text":"ship"}]}"#;
    let s = steps(args).unwrap();
    assert_eq!(s[0].text, "write the docs", "whitespace folds to one line");
    assert_eq!(
        (s[0].state, s[1].state),
        (StepState::Done, StepState::Pending)
    );
}

#[test]
fn a_step_without_words_is_dropped_and_a_long_one_is_cut() {
    let long = "x".repeat(MAX_CHARS + 40);
    let args = format!(r#"{{"items":[{{"text":"  "}},{{"text":"{long}"}}]}}"#);
    let s = steps(&args).unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].text.chars().count(), MAX_CHARS);
}

#[test]
fn something_that_is_not_a_list_is_none() {
    assert!(steps("not json").is_none());
    assert!(steps(r#"{"path":"a.rs"}"#).is_none());
    assert_eq!(
        steps(r#"{"todos":[]}"#),
        Some(Vec::new()),
        "an emptied list is a list"
    );
}

#[test]
fn only_the_two_list_tools_count() {
    assert!(is_list("sys:todo") && is_list("TodoWrite"));
    assert!(!is_list("Write") && !is_list("sys:run"));
}

#[test]
fn a_step_round_trips_the_wire() {
    let s = Step {
        text: "run it".into(),
        state: StepState::Active,
    };
    let j = serde_json::to_string(&s).unwrap();
    assert_eq!(j, r#"{"text":"run it","state":"active"}"#);
    assert_eq!(serde_json::from_str::<Step>(&j).unwrap(), s);
}
