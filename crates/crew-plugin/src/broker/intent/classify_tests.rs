//! `complete_once` is called from sync code that may itself be inside a
//! runtime: the session log folds its overflow through it from the event
//! path, and a swarm's events go out from inside its own `block_on`.
use std::sync::Arc;
use std::time::Duration;

use super::complete_once;

#[test]
fn a_completion_asked_for_inside_a_runtime_answers_instead_of_panicking() {
    let provider: Arc<dyn crew_hive::Provider> = Arc::new(crew_hive::MockProvider {
        reply: "folded".into(),
    });
    let ask = || complete_once(&provider, "m", "fold this", 16, Duration::from_secs(5));
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    // The swarm's shape: a sync call made while its own `block_on` runs.
    // This was "Cannot start a runtime from within a runtime".
    let inside = rt.block_on(async { ask() });
    assert_eq!(inside.map(|c| c.text), Ok("folded".to_string()));
    // …and from plain sync code, as it always worked.
    assert_eq!(ask().map(|c| c.text), Ok("folded".to_string()));
}
