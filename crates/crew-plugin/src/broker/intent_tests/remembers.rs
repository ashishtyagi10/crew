//! The router's `REMEMBER:` line is a standing note: said once, it is in
//! the memory file and in front of the next task, and the pane says so.
use super::capability::seed_repo;
use super::gate::led;
use super::*;

fn collect() -> (
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

fn notes() -> String {
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::read_to_string(dir.join(".crew").join("memory.md")).unwrap_or_default()
}

#[test]
fn the_remember_line_is_parsed_and_a_hedge_is_not_a_note() {
    let h = |r: &str| super::super::hints::Hints::parse(r, &[]).remember;
    let reply = "SHAPE: reply\nREMEMBER: talk like Agent Smith.";
    assert_eq!(h(reply).as_deref(), Some("talk like Agent Smith"));
    assert_eq!(h("SHAPE: reply\nREMEMBER: none"), None);
    assert_eq!(h("SHAPE: reply"), None);
}

#[test]
fn a_routed_preference_is_written_said_and_carried_by_the_next_task() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let (said, mut emit) = collect();
    let call = |_: &str| Ok("SHAPE: reply\nREMEMBER: talk like Agent Smith".to_string());
    let tick = crate::broker::tick::noop_tick_emit();
    let task = "you should be talking like Agent Smith";
    route_with(task, Some(&call), &mut session, &tick, &mut emit).unwrap();
    assert_eq!(notes(), "- talk like Agent Smith\n");
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("remembered for every task"), "{text}");
    let next = crate::broker::memory::with_memory("hello", "hello");
    assert!(next.contains("talk like Agent Smith"), "{next}");
}

#[test]
fn a_router_that_names_no_preference_writes_none() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let call = |_: &str| Ok("SHAPE: reply".to_string());
    let tick = crate::broker::tick::noop_tick_emit();
    let task = "always wondered how src/gen.rs is made, explain it";
    route_with(task, Some(&call), &mut session, &tick, &mut |_| Ok(())).unwrap();
    assert_eq!(notes(), "");
}

#[test]
fn a_led_preference_is_kept_by_its_opening() {
    let _g = testenv::mock_with_specialists("ok\n@done", testenv::TRIO);
    seed_repo();
    let mut session = Session::new();
    let tick = crate::broker::tick::noop_tick_emit();
    let (said, mut emit) = collect();
    let task = "from now on answer in one paragraph";
    led(task, "planner", &mut session, &tick, &mut emit).unwrap();
    assert_eq!(notes(), "- from now on answer in one paragraph\n");
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("remembered for every task"), "{text}");
}
