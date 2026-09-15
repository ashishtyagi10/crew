use super::*;

use std::sync::atomic::Ordering;

fn capture() -> (
    std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    impl FnMut(PluginEvent) -> anyhow::Result<()>,
) {
    let said = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = said.clone();
    let emit = move |ev: PluginEvent| {
        if let PluginEvent::Message { text, .. } = &ev {
            sink.lock().unwrap().push(text.clone());
        }
        Ok(())
    };
    (said, emit)
}

fn failed() -> Outcome {
    outcome(Ok(
        "exit 101\nerror[E0308]: mismatched types\n  --> src/a.rs:4:9\n".into(),
    ))
}

#[test]
fn the_pass_is_handed_the_command_the_output_and_one_instruction() {
    let task = repair_task("cargo test", &failed());
    assert!(task.contains("Command: cargo test"), "{task}");
    assert!(task.contains("error[E0308]"), "{task}");
    assert!(task.contains("do not weaken or delete"), "{task}");
    assert!(task.contains("do not commit"), "{task}");
}

#[test]
fn a_pass_runs_once_and_never_starts_another_from_inside_itself() {
    let session = Session::default();
    let (said, mut emit) = capture();
    let asked = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen = asked.clone();
    // The repair itself asks for another pass — the shape a real one has,
    // since fixing the build changes files, which runs the check again.
    let mut repair = move |t: &str| {
        seen.lock().unwrap().push(t.to_string());
        let inner = Session::default();
        inner.repairing.store(true, Ordering::Relaxed);
        Ok(())
    };
    // `false` as the command, not a real build: the pass re-runs the check
    // itself, and a test that shells out to `cargo test` would run this
    // suite inside itself.
    take_one_pass(&session, "false", &failed(), &mut repair, &mut emit).unwrap();
    assert_eq!(asked.lock().unwrap().len(), 1, "more than one pass ran");
    assert!(!session.repairing.load(Ordering::Relaxed), "the flag stuck");
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("taking one pass"), "{text}");
    assert!(
        text.contains("one pass was not enough"),
        "a still-failing check must say so: {text}"
    );
}

#[test]
fn a_session_already_repairing_takes_no_pass_at_all() {
    let session = Session::default();
    session.repairing.store(true, Ordering::Relaxed);
    let (said, mut emit) = capture();
    let mut repair = |_: &str| panic!("a second pass was started");
    take_one_pass(&session, "false", &failed(), &mut repair, &mut emit).unwrap();
    assert!(said.lock().unwrap().is_empty());
}

#[test]
fn the_user_can_say_no_and_then_say_yes_again() {
    assert_eq!(asks("don't fix it yourself"), Some(false));
    assert_eq!(asks("Dont fix it yourself."), Some(false));
    assert_eq!(asks("fix it yourself"), Some(true));
    assert_eq!(
        asks("fix the router yourself"),
        None,
        "a task is not a switch"
    );
    let session = Session::default();
    let (_said, mut emit) = capture();
    gate("don't fix it yourself", &session, &mut emit)
        .unwrap()
        .unwrap();
    assert!(!autofix(&session));
    let mut repair = |_: &str| panic!("a pass ran after the user said not to");
    take_one_pass(&session, "false", &failed(), &mut repair, &mut emit).unwrap();
    gate("fix it yourself", &session, &mut emit)
        .unwrap()
        .unwrap();
    assert!(autofix(&session));
}
