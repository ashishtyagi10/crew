use super::*;

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

#[test]
fn the_mode_is_turned_on_and_off_in_words() {
    for on in [
        "plan first",
        "Plan Mode",
        "plan mode on",
        "always plan first.",
    ] {
        assert_eq!(asks(on), Some(Ask::On), "{on:?}");
    }
    for off in ["stop planning", "plan mode off", "no more plans"] {
        assert_eq!(asks(off), Some(Ask::Off), "{off:?}");
    }
}

#[test]
fn a_task_that_merely_mentions_planning_is_a_task() {
    for task in [
        "plan first, then write the migration",
        "draft a plan for the router",
        "stop planning and start writing tests",
        "what is the plan mode for",
    ] {
        assert_eq!(asks(task), None, "{task:?} was taken as a mode switch");
    }
}

#[test]
fn the_gate_flips_the_session_and_says_which_way() {
    let session = Session::default();
    let (said, mut emit) = capture();
    assert!(!on(&session), "plan-first must start off");
    gate("plan first", &session, &mut emit)
        .expect("a switch")
        .unwrap();
    assert!(on(&session));
    gate("stop planning", &session, &mut emit)
        .expect("a switch")
        .unwrap();
    assert!(!on(&session));
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("plan first is ON"), "{text}");
    assert!(text.contains("plan first is OFF"), "{text}");
    assert!(
        text.contains("approve"),
        "the ON line must say how to run it"
    );
}

#[test]
fn an_ordinary_task_is_not_a_switch() {
    let session = Session::default();
    let (_said, mut emit) = capture();
    assert!(gate("add a test for the router", &session, &mut emit).is_none());
}

#[test]
fn while_the_mode_is_on_every_plain_task_is_routed_to_the_plan_gate() {
    let session = Session::default();
    let (said, mut emit) = capture();
    session
        .plan_first
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let d =
        crate::broker::intent::decision::forced(crate::broker::intent::Shape::Plan, WHY, &mut emit)
            .unwrap();
    assert_eq!(d.shape, crate::broker::intent::Shape::Plan);
    let text = said.lock().unwrap().join("\n");
    assert_eq!(text, "routing: plan — plan first is on", "{text}");
}
