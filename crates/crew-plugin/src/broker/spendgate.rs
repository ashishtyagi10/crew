//! The cap held OUTSIDE a bounded call, for the callers that cannot wrap a
//! provider: the fan's registry agents (built at startup, before any task)
//! and the one-shots a bound surrounds (the judges' 30 s, the closing
//! answer's 3 min).
//!
//! WHY outside: `spendcapwrap::Capped` asks from INSIDE the call, so a bound
//! around it counts the question's wait (up to five minutes) as the model's
//! time, times out mid-question and leaves the pane asking one nobody is
//! waiting on. A gate asks first, before the bound starts, and charges after.
use std::sync::Arc;

use super::adapter::Usage;
use super::spendcap::{self, TaskSpend};

/// The running task's meter, taken on the TASK's thread and carried to
/// wherever the call runs. Empty when no capped task is running.
#[derive(Clone)]
pub(crate) struct Gate(Option<Arc<TaskSpend>>);

impl Gate {
    /// The gate of the task on this thread: call it on the task's thread.
    pub(crate) fn here() -> Self {
        Self(spendcap::current())
    }

    /// Wait until the task may spend: `Err` when it was stopped at the cap or
    /// by `/stop`. The wait ends with the answer, the approval clock or the
    /// stop, so it cannot hang.
    pub(crate) fn before(&self) -> Result<(), String> {
        let Some(task) = &self.0 else { return Ok(()) };
        // Under the cap — almost every call — nothing to ask: no thread, no
        // runtime (`before_call` would say the same, after paying for both).
        if !task.over() {
            return Ok(());
        }
        // A thread of its own when called from inside a runtime (a swarm's
        // judge runs inside the swarm's `block_on`), as `complete_once` does:
        // there a nested `block_on` is a panic, not a wait.
        let ask = |task: &Arc<TaskSpend>| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            rt.block_on(task.before_call())
        };
        match tokio::runtime::Handle::try_current().is_ok() {
            true => std::thread::scope(|s| {
                s.spawn(|| ask(task))
                    .join()
                    .unwrap_or_else(|_| Err("the spend check panicked".into()))
            }),
            false => ask(task),
        }
    }

    /// A finished call's cost joins the task's.
    pub(crate) fn after(&self, micros: u64) {
        if let Some(task) = &self.0 {
            task.add(micros);
        }
    }

    /// One blocking agent call (`fan_out`'s threads) under the gate.
    pub(crate) fn hold(
        &self,
        call: impl FnOnce() -> Result<(String, Usage), String>,
    ) -> Result<(String, Usage), String> {
        self.before()?;
        let done = call()?;
        self.after(done.1.cost_microusd);
        Ok(done)
    }
}
