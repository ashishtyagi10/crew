use super::*;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}
fn never() -> String {
    panic!("stdin must not be read")
}

#[test]
fn a_prompt_argument_is_used_and_stdin_is_left_alone() {
    let o = parse(&s(&["fix", "the typo"]), never).unwrap();
    assert_eq!(
        o,
        Opts {
            prompt: "fix the typo".into(),
            allow: false,
            timeout: None,
        }
    );
}

#[test]
fn a_dash_or_no_prompt_reads_stdin() {
    assert_eq!(
        parse(&s(&["-"]), || "from pipe\n".into()).unwrap().prompt,
        "from pipe"
    );
    assert_eq!(parse(&[], || "x".into()).unwrap().prompt, "x");
}

#[test]
fn yes_and_allow_edits_both_allow() {
    assert!(parse(&s(&["--yes", "go"]), never).unwrap().allow);
    assert!(parse(&s(&["go", "--allow-edits"]), never).unwrap().allow);
}

#[test]
fn no_prompt_anywhere_is_a_usage_error() {
    assert!(parse(&[], String::new).is_err());
    assert!(parse(&s(&["--yes"]), || "  \n".into()).is_err());
}

#[test]
fn an_unknown_flag_is_a_usage_error_but_after_dashdash_it_is_prompt() {
    assert!(parse(&s(&["--bogus", "x"]), never).is_err());
    assert_eq!(
        parse(&s(&["--", "--bogus"]), never).unwrap().prompt,
        "--bogus"
    );
}

#[test]
fn timeout_takes_whole_seconds_and_refuses_anything_else() {
    let o = parse(&s(&["--timeout", "90", "go"]), never).unwrap();
    assert_eq!(o.timeout, Some(std::time::Duration::from_secs(90)));
    assert_eq!(o.prompt, "go");
    for bad in [
        &["--timeout"][..],
        &["--timeout", "soon", "go"],
        &["--timeout", "0", "go"],
    ] {
        assert!(parse(&s(bad), never).is_err(), "{bad:?}");
    }
}
