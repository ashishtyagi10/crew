use super::*;

#[test]
fn raw_normalize_trims() {
    assert_eq!(Normalize::Raw.apply("  PONG\n"), "PONG");
}

#[test]
fn build_args_substitutes_body() {
    let a = CliAdapter {
        name: "x".into(),
        program: "echo".into(),
        args: vec!["-p".into(), "{}".into()],
        normalize: Normalize::Raw,
    };
    assert_eq!(a.build_args("hi there"), vec!["-p", "hi there"]);
}

#[test]
fn cli_adapter_calls_real_process() {
    // `cat` echoes its arg back via a shell so we exercise call()+normalize.
    let a = CliAdapter {
        name: "echoer".into(),
        program: "sh".into(),
        args: vec!["-c".into(), "printf %s \"$0\"".into(), "{}".into()],
        normalize: Normalize::Raw,
    };
    assert_eq!(a.call("hello", Duration::from_secs(5)).unwrap(), "hello");
}
