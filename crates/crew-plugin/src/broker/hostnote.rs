//! One door for the notes background loaders owe the host: a skill file
//! that would not read, an agent manifest that would not parse. Those
//! loaders are free functions reached from a dozen call sites (`/skills`,
//! `/doctor`, every relay and swarm turn), none of which owns an emitter —
//! so, like the approval gate's emitter, the host's `Status` sink is
//! process-wide, set once by the stdio loop, and silent (not failing) in
//! every other host. The alternative — threading `(Vec<Skill>, Vec<String>)`
//! up through `skills::list`, which the GUI's attach picker also calls — put
//! the error path in front of callers that have nowhere to send it.
//!
//! NOTE FOR TESTS: no unit test may install the sink. It is a `OnceLock`
//! shared by every test in the process; the loaders take an explicit error
//! callback (`load_dir_with`) for exactly that reason.
use std::sync::{Arc, OnceLock};

use crate::PluginEvent;

type Sink = Arc<dyn Fn(PluginEvent) + Send + Sync>;

static SINK: OnceLock<Sink> = OnceLock::new();

/// Point loader notes at the host. Idempotent; the first caller wins.
pub(crate) fn set_sink(f: Sink) {
    let _ = SINK.set(f);
}

/// One `Status` line for the host's LOG; dropped when no host is listening.
pub(crate) fn status(error: bool, message: impl Into<String>) {
    if let Some(sink) = SINK.get() {
        sink(PluginEvent::Status {
            error,
            message: message.into(),
        });
    }
}
