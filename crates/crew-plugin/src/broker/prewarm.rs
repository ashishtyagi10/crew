//! Opening the provider connection while the user is still typing.
//!
//! The app sends `PluginCommand::Warm` on the first keystroke of a message.
//! The turn that follows opens with a model call — the router's
//! classification, on the provider `discover` resolves — and measured, that
//! first call paid ~0.5 s of TCP and TLS whenever the pooled socket had idled
//! past its 30 s. Codex and Claude Code hide the handshake by connecting early;
//! this is the same move, made on a hint from the composer.
//!
//! What it costs: one bodiless HEAD to the provider host per 20 s of composing
//! at most, with no key and no tokens — and nothing while nobody types.
//! `CREW_PREWARM=0` turns it off.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// At most one warm this often. The pool keeps an idle socket 30 s, so a warm
/// 20 s old still has 10 s to live when the message is sent; warming more
/// often only repeats a handshake that is already done.
pub(super) const EVERY: Duration = Duration::from_secs(20);

/// When this broker last warmed. One per broker process — which is one per
/// pane — so two panes typing at once each warm their own.
#[derive(Default)]
pub(super) struct Latch {
    last: Option<Instant>,
}

impl Latch {
    /// Whether a warm goes out at `now`, recording it when one does.
    ///
    /// Not while a task runs: its calls already hold a connection to the same
    /// host, and a turn typed behind it is queued until it ends. A refusal
    /// for that reason records nothing, so the first key after the task has
    /// finished still warms.
    pub(super) fn admit(&mut self, now: Instant, busy: bool) -> bool {
        if busy {
            return false;
        }
        if self
            .last
            .is_some_and(|t| now.saturating_duration_since(t) < EVERY)
        {
            return false;
        }
        self.last = Some(now);
        true
    }
}

/// `CREW_PREWARM`'s verdict: on unless it is exactly `0`, like `CREW_INTENT`.
pub(super) fn enabled(var: Option<&str>) -> bool {
    var != Some("0")
}

/// The stdin loop's `Warm` arm. Never waits: discovery reads the credential
/// store and may probe a CLI's sign-in, so the provider is resolved and
/// warmed on a thread of its own while the loop goes back to stdin.
pub(super) fn on_warm(tasks: &mut super::tasks::Tasks) {
    static LATCH: Mutex<Latch> = Mutex::new(Latch { last: None });
    if !enabled(std::env::var("CREW_PREWARM").ok().as_deref()) {
        return;
    }
    tasks.reap();
    let busy = tasks.len() > 0;
    let mut latch = LATCH.lock().unwrap_or_else(|e| e.into_inner());
    if latch.admit(Instant::now(), busy) {
        let _ = std::thread::Builder::new()
            .name("crew-prewarm".into())
            .spawn(warm_next_provider);
    }
}

/// Warm the provider the next call will use. `Cheap` because the turn's
/// first call is the router's, which asks for that tier — every tier of a
/// provider posts to the same host, so the socket serves the answer's call
/// too.
fn warm_next_provider() -> bool {
    warm_resolved(super::discover::provider_and_model_for(
        crew_hive::ModelTier::Cheap,
    ))
}

/// Warm what discovery resolved, if it resolved something with a socket to
/// open — not nothing (no key), and not the mock, which the GUI harness runs
/// on and which has no host. Returns whether a warm was asked for.
pub(super) fn warm_resolved(found: Option<(Arc<dyn crew_hive::Provider>, String)>) -> bool {
    match found {
        Some((provider, model)) if model != "mock" => {
            provider.warm();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "prewarm_tests.rs"]
mod tests;
