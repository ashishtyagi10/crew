use super::*;
use crew_hive::MockProvider;

fn mock(reply: &str) -> Arc<dyn Provider> {
    Arc::new(MockProvider {
        reply: reply.to_string(),
    })
}

#[test]
fn call_returns_the_providers_trimmed_reply() {
    let a = ApiAdapter::new("planner", "m", "", None, mock("  do this\n@done  ")).unwrap();
    assert_eq!(
        a.call("task", Duration::from_secs(5)).unwrap(),
        "do this\n@done"
    );
}

#[test]
fn a_specialist_reports_its_own_role() {
    let a = ApiAdapter::specialist("archivist", "records, retrieval", "m", mock("hi")).unwrap();
    assert_eq!(a.name(), "archivist");
    assert_eq!(a.role(), "records, retrieval");
}

#[test]
fn a_specialists_system_prompt_carries_its_name_and_role() {
    let a = ApiAdapter::specialist("archivist", "records, retrieval", "m", mock("hi")).unwrap();
    let sys = a
        .system
        .clone()
        .expect("specialists always get a system prompt");
    assert!(sys.contains("archivist"), "got {sys}");
    assert!(sys.contains("records, retrieval"), "got {sys}");
}

#[test]
fn a_roleless_specialist_still_gets_a_usable_prompt() {
    // expertise is allowed to be empty; the prompt must not read as
    // "Your specialty is ." in that case.
    let a = ApiAdapter::specialist("mystery", "", "m", mock("hi")).unwrap();
    let sys = a.system.clone().unwrap();
    assert!(sys.contains("mystery"), "got {sys}");
    assert!(!sys.contains("specialty is ."), "got {sys}");
}

#[test]
fn ticked_call_reports_growing_char_estimates() {
    // MockProvider streams ~3 chunks; the estimator must report a
    // non-decreasing chars/4 sequence and the final text must match.
    let adapter = ApiAdapter::new(
        "planner",
        "m",
        "",
        None,
        mock("one two three four five six"),
    )
    .unwrap();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
    let sink = seen.clone();
    let on_tokens: Arc<dyn Fn(u64) + Send + Sync> = Arc::new(move |t| {
        sink.lock().unwrap().push(t);
    });
    let stream = HopStream {
        on_tokens,
        on_text: Arc::new(|_| {}),
    };
    let (text, _usage) = adapter
        .call_with_usage_ticked("task", Duration::from_secs(5), &stream)
        .unwrap();
    assert_eq!(text, "one two three four five six");
    let ticks = seen.lock().unwrap();
    assert!(ticks.len() >= 2, "mock streams >=2 chunks: {ticks:?}");
    assert!(
        ticks.windows(2).all(|w| w[0] <= w[1]),
        "estimates never shrink"
    );
    let total_chars = "one two three four five six".len() as u64;
    assert_eq!(
        *ticks.last().unwrap(),
        total_chars / 4,
        "final estimate = chars/4"
    );
}

#[test]
fn ticked_estimates_count_chars_not_bytes() {
    // 8 CJK chars = 24 UTF-8 bytes: bytes/4 would report 6, chars/4
    // must report 2 (same convention as the provider-side estimators).
    let adapter = ApiAdapter::new("planner", "m", "", None, mock("文文文文 文文文文")).unwrap();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
    let sink = seen.clone();
    let on_tokens: Arc<dyn Fn(u64) + Send + Sync> = Arc::new(move |t| {
        sink.lock().unwrap().push(t);
    });
    let stream = HopStream {
        on_tokens,
        on_text: Arc::new(|_| {}),
    };
    let (text, _usage) = adapter
        .call_with_usage_ticked("task", Duration::from_secs(5), &stream)
        .unwrap();
    assert_eq!(text, "文文文文 文文文文");
    let ticks = seen.lock().unwrap();
    let total_chars = "文文文文 文文文文".chars().count() as u64; // 9
    assert_eq!(
        *ticks.last().unwrap(),
        total_chars / 4,
        "final estimate uses chars: {ticks:?}"
    );
}
