//! The router's `CORRECTS:` line is memory: a correction routed today is in
//! front of tomorrow's task on the same file, and nowhere else.
use super::capability::seed_repo;
use super::gate::led;
use super::*;
use crate::broker::recall::lock;

fn routed(session: &mut Session, task: &str, reply: &'static str) {
    let call = move |_: &str| Ok(reply.to_string());
    route_with(
        task,
        Some(&call),
        session,
        &crate::broker::tick::noop_tick_emit(),
        &mut |_| Ok(()),
    )
    .unwrap();
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
fn a_routed_correction_reaches_a_later_task_on_its_file_only() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let fix = "regenerate src/gen.rs from the schema instead";
    routed(
        &mut session,
        fix,
        "SHAPE: reply\nCORRECTS: hand-editing src/gen.rs",
    );
    let later = told(&session, "add a field to src/gen.rs");
    assert!(
        later.contains("ruled out before (just now): hand-editing src/gen.rs"),
        "{later}"
    );
    assert!(!told(&session, "what is the weather in berlin").contains("ruled out"));
}

#[test]
fn a_router_that_says_no_correction_writes_none() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    routed(
        &mut session,
        "don't stop at src/gen.rs, also do src/api.rs",
        "SHAPE: reply",
    );
    assert!(!told(&session, "edit src/gen.rs").contains("ruled out"));
}

#[test]
fn a_led_correction_is_remembered_by_its_opening() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let tick = crate::broker::tick::noop_tick_emit();
    let task = "no, don't touch src/gen.rs, it is generated";
    led(task, "planner", &mut session, &tick, &mut |_| Ok(())).unwrap();
    let later = told(&session, "rename a type in src/gen.rs");
    assert!(later.contains("ruled out before"), "{later}");
}
