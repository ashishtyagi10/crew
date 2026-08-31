//! The liveness/verdict engine for inter-pane `ask`: a pure state machine fed
//! one observation per poll tick. It keeps waiting while the target genuinely
//! emits output, and resolves to a verdict when the sentinel closes, the
//! target goes idle, or it stalls (silent past an adaptive budget). All time
//! is passed in as `now_ms` — no `Instant::now` here — so it's fully testable
//! over scripted sequences.

/// Base patience for an active-but-silent target, in ms. The full budget is
/// this plus how long the target has been streaming (a long stream earns more).
const BASE_QUIET_MS: u64 = 4_000;

/// A registered ask waiting on the target pane.
pub(crate) struct PendingAsk {
    pub id: String,
    pub target: usize,
    /// The tick this ask was registered — the base for an absolute ceiling.
    pub asked_ms: u64,
    pub captured: String,
    pub produced_any: bool,
    pub first_out_ms: Option<u64>,
    pub last_progress_ms: u64,
}

/// One tick's observation of the target pane.
pub(crate) struct Obs<'a> {
    /// Newly-emitted output from the target since the last tick (decoded).
    pub new_output: &'a str,
    /// The target returned to its idle prompt / ended its turn this tick.
    pub idle_transition: bool,
    pub now_ms: u64,
}

/// What the engine decides after an observation.
pub(crate) enum Step {
    Wait,
    Answered(String),
    /// Produced output but never closed the sentinel; carries any partial.
    Stalled(Option<String>),
    /// Went idle having produced nothing addressable.
    IdleNoEngage,
}

impl PendingAsk {
    pub(crate) fn new(id: String, target: usize, now_ms: u64) -> Self {
        PendingAsk {
            id,
            target,
            asked_ms: now_ms,
            captured: String::new(),
            produced_any: false,
            first_out_ms: None,
            last_progress_ms: now_ms,
        }
    }

    pub(crate) fn observe(&mut self, o: Obs) -> Step {
        if !o.new_output.is_empty() {
            self.captured.push_str(o.new_output);
            self.produced_any = true;
            self.first_out_ms.get_or_insert(o.now_ms);
            self.last_progress_ms = o.now_ms;
        }
        // A closed sentinel is the answer, regardless of anything else.
        if let Some(text) = crate::askroute::scan_answer(&self.captured, &self.id) {
            return Step::Answered(text);
        }
        // Target's turn ended without a sentinel: engaged-but-incomplete, or
        // never engaged at all.
        if o.idle_transition {
            return if self.produced_any {
                Step::Stalled(Some(self.captured.clone()))
            } else {
                Step::IdleNoEngage
            };
        }
        // Still active but silent: keep waiting until an adaptive budget — the
        // longer it actually streamed (first output → last output), the more
        // patience it earns. (Span is the streaming duration, NOT time-since-
        // start, so a single early burst then silence earns only the base.)
        if self.produced_any {
            let span = self
                .last_progress_ms
                .saturating_sub(self.first_out_ms.unwrap_or(self.last_progress_ms));
            let budget = BASE_QUIET_MS + span;
            if o.now_ms.saturating_sub(self.last_progress_ms) > budget {
                return Step::Stalled(Some(self.captured.clone()));
            }
        }
        Step::Wait
    }
}

#[cfg(test)]
#[path = "askwait_tests.rs"]
mod tests;
