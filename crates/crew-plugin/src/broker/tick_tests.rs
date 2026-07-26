use super::*;
use std::sync::{Arc, Mutex};

/// Serialises the two `hop_texter` tests below against each other's
/// `CREW_STREAM_TEXT` mutation — the var is process-global and Rust runs
/// tests in parallel, so a bare `set_var`/`remove_var` would race any test
/// that reads it concurrently. Mirrors `crew-app::envlock::with_home`.
static STREAM_TEXT_LOCK: Mutex<()> = Mutex::new(());

type TickEmit = Arc<dyn Fn(PluginEvent) + Send + Sync>;

#[test]
fn first_tick_passes_then_gap_enforced() {
    assert!(should_tick(None, 0, TICK_GAP_MS));
    assert!(!should_tick(Some(1000), 1149, TICK_GAP_MS));
    assert!(should_tick(Some(1000), 1150, TICK_GAP_MS));
    assert!(
        !should_tick(Some(1000), 999, TICK_GAP_MS),
        "clock skew saturates, no panic"
    );
}

#[test]
fn text_gate_first_push_flushes_immediately() {
    let mut g = TextGate::new();
    assert_eq!(g.push("hello", 0).as_deref(), Some("hello"));
}

#[test]
fn text_gate_buffers_inside_the_gap_and_loses_no_character() {
    // This is the one way TextGate MUST differ from should_tick's numeric
    // gate: a token estimate is monotonic so a skipped tick self-heals,
    // but text is cumulative — a fragment inside the gap has to be kept.
    let mut g = TextGate::new();
    let mut flushed = String::new();
    for (frag, t) in [("a", 0u64), ("b", 10), ("c", 40), ("d", 70), ("e", 100)] {
        if let Some(p) = g.push(frag, t) {
            flushed.push_str(&p);
        }
    }
    assert_eq!(flushed, "abcde", "every character reached a flush");
}

#[test]
fn text_gate_never_flushes_an_empty_payload() {
    let mut g = TextGate::new();
    assert_eq!(g.push("", 0), None);
    // …and the suppressed empty push must not consume the first-flush
    // allowance, or the next real fragment would be delayed a full gap.
    assert_eq!(g.push("x", 1).as_deref(), Some("x"));
}

#[test]
fn text_gate_survives_clock_skew() {
    let mut g = TextGate::new();
    assert_eq!(g.push("a", 1000).as_deref(), Some("a"));
    assert_eq!(g.push("b", 999), None, "saturating subtraction, no panic");
}

/// Collect emitted events into a shared `Vec`, standing in for `host.rs`'s
/// wire writer.
fn recording_tick_emit() -> (TickEmit, Arc<Mutex<Vec<PluginEvent>>>) {
    let events: Arc<Mutex<Vec<PluginEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = events.clone();
    let emit: Arc<dyn Fn(PluginEvent) + Send + Sync> = Arc::new(move |ev| {
        sink.lock().unwrap_or_else(|e| e.into_inner()).push(ev);
    });
    (emit, events)
}

#[test]
fn hop_texter_emits_exactly_one_delta_carrying_the_fragment() {
    let _g = STREAM_TEXT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var_os("CREW_STREAM_TEXT");
    std::env::remove_var("CREW_STREAM_TEXT"); // default: streaming enabled

    let (emit, events) = recording_tick_emit();
    let on_text = hop_texter(emit, "coder".to_string());
    on_text("hello");

    match prev {
        Some(p) => std::env::set_var("CREW_STREAM_TEXT", p),
        None => std::env::remove_var("CREW_STREAM_TEXT"),
    }

    let got = events.lock().unwrap_or_else(|e| e.into_inner());
    assert_eq!(got.len(), 1, "expected exactly one Delta, got {got:?}");
    match &got[0] {
        PluginEvent::Delta { agent, text } => {
            assert_eq!((agent.as_str(), text.as_str()), ("coder", "hello"));
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn hop_texter_emits_nothing_when_stream_text_env_is_zero() {
    let _g = STREAM_TEXT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prev = std::env::var_os("CREW_STREAM_TEXT");
    std::env::set_var("CREW_STREAM_TEXT", "0");

    // hop_texter reads the env var once, at construction — set it first.
    let (emit, events) = recording_tick_emit();
    let on_text = hop_texter(emit, "coder".to_string());
    on_text("hello");

    match prev {
        Some(p) => std::env::set_var("CREW_STREAM_TEXT", p),
        None => std::env::remove_var("CREW_STREAM_TEXT"),
    }

    assert!(
        events.lock().unwrap_or_else(|e| e.into_inner()).is_empty(),
        "CREW_STREAM_TEXT=0 must suppress every Delta"
    );
}
