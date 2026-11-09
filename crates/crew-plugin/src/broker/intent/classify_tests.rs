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
    let ask = || {
        complete_once(
            &provider,
            "m",
            "fold this",
            16,
            Duration::from_secs(5),
            None,
        )
    };
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

/// Every one-shot but the summary (the reply judge, the router, the
/// classifiers) goes through `live_bounded`, which asks for low effort; `crew_hive`'s `effort_tests` prove what that puts on the wire.
#[test]
fn a_one_shot_asks_for_low_effort() {
    use std::sync::Mutex;
    struct Seen(Mutex<Option<crew_hive::CompletionRequest>>);
    impl crew_hive::Provider for Seen {
        fn complete(
            &self,
            req: crew_hive::CompletionRequest,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<
                        Output = Result<crew_hive::Completion, crew_hive::ProviderError>,
                    > + Send,
            >,
        > {
            *self.0.lock().unwrap() = Some(req);
            Box::pin(async { Ok(crew_hive::Completion::default()) })
        }
    }
    let seen = Arc::new(Seen(Mutex::new(None)));
    let provider: Arc<dyn crew_hive::Provider> = seen.clone();
    complete_once(
        &provider,
        "claude-opus-5-5",
        "judge",
        16,
        Duration::from_secs(5),
        super::ONESHOT_EFFORT,
    )
    .unwrap();
    let req = seen
        .0
        .lock()
        .unwrap()
        .take()
        .expect("the call reached the provider");
    assert_eq!(req.effort, Some(crew_hive::Effort::Low));
}
