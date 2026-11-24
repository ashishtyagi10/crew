//! The router's `CORRECTS:` line is memory: a correction routed today is
//! proposed in one line, and once the user says "keep" it is in front of
//! tomorrow's task on the same file, and nowhere else.
use super::capability::seed_repo;
use super::gate::led;
use super::*;
use crate::broker::recall::lock;

/// Route `task` as production does, and return every line the pane got.
fn routed(session: &mut Session, task: &str, reply: &'static str) -> Vec<String> {
    let call = move |_: &str| Ok(reply.to_string());
    let mut said = Vec::new();
    route_with(
        task,
        Some(&call),
        session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |e| {
            if let crate::PluginEvent::Message { text, .. } = e {
                said.push(text);
            }
            Ok(())
        },
    )
    .unwrap();
    said
}

/// The lines that propose a correction.
fn proposals(said: &[String]) -> usize {
    said.iter()
        .filter(|l| l.starts_with("correction noted, not kept yet"))
        .count()
}

fn told(session: &Session, task: &str) -> String {
    lock(&session.recall).context(task, &[]).unwrap_or_default()
}

#[test]
fn the_corrects_line_is_parsed_and_a_hedge_is_not_a_correction() {
    let h = |r: &str| super::super::hints::Hints::parse(r, &[]).corrects;
    let reply = "SHAPE: reply\nCORRECTS: hand-editing src/gen.rs.";
    assert_eq!(h(reply).as_deref(), Some("hand-editing src/gen.rs"));
    assert_eq!(h("SHAPE: reply\nCORRECTS: none"), None);
    assert_eq!(h("SHAPE: reply"), None);
}

#[test]
fn a_routed_correction_is_proposed_and_reaches_a_later_task_only_when_kept() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let fix = "regenerate src/gen.rs from the schema instead";
    let said = routed(
        &mut session,
        fix,
        "SHAPE: reply\nCORRECTS: hand-editing src/gen.rs",
    );
    assert_eq!(proposals(&said), 1, "{said:?}");
    let later = "add a field to src/gen.rs";
    assert!(!told(&session, later).contains("ruled out"), "not kept yet");
    let kept = routed(&mut session, "keep", "SHAPE: reply");
    assert!(
        kept.iter()
            .any(|l| l.starts_with("kept: hand-editing src/gen.rs")),
        "{kept:?}"
    );
    assert!(
        told(&session, later).contains("ruled out before (just now): hand-editing src/gen.rs"),
        "{}",
        told(&session, later)
    );
    assert!(!told(&session, "what is the weather in berlin").contains("ruled out"));
}

#[test]
fn a_forgotten_correction_is_never_kept() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let fix = "regenerate src/gen.rs from the schema instead";
    routed(
        &mut session,
        fix,
        "SHAPE: reply\nCORRECTS: hand-editing src/gen.rs",
    );
    let gone = routed(&mut session, "forget", "SHAPE: reply");
    assert!(
        gone.iter().any(|l| l.starts_with("forgot: hand-editing")),
        "{gone:?}"
    );
    assert!(!told(&session, "add a field to src/gen.rs").contains("ruled out"));
    assert!(lock(&session.recall).ruled().is_empty());
    assert_eq!(lock(&session.recall).proposed(), None);
}

#[test]
fn a_router_that_says_no_correction_writes_none() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let said = routed(
        &mut session,
        "don't stop at src/gen.rs, also do src/api.rs",
        "SHAPE: reply",
    );
    assert_eq!(proposals(&said), 0, "the router read it: {said:?}");
    assert_eq!(lock(&session.recall).proposed(), None);
    assert!(!told(&session, "edit src/gen.rs").contains("ruled out"));
}

#[test]
fn a_led_correction_is_proposed_by_its_opening_and_kept_on_keep() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let tick = crate::broker::tick::noop_tick_emit();
    let task = "no, don't touch src/gen.rs, it is generated";
    let mut said = Vec::new();
    led(task, "planner", &mut session, &tick, &mut |e| {
        if let crate::PluginEvent::Message { text, .. } = e {
            said.push(text);
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(proposals(&said), 1, "{said:?}");
    let later = "rename a type in src/gen.rs";
    assert!(!told(&session, later).contains("ruled out"));
    led("keep", "planner", &mut session, &tick, &mut |_| Ok(())).unwrap();
    assert!(
        told(&session, later).contains("ruled out before"),
        "{}",
        told(&session, later)
    );
}
