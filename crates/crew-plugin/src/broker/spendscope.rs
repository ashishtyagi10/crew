//! What a task's spend cap is made of outside the meter: the dollars an
//! environment value means, how they read, and the thread-local the task's
//! meter travels in.
//!
//! WHY apart from the meter (`spendcap`): the parsing and the scope are what
//! the entry points touch (`stdio` enters it, `spendcapwrap` and `fan` read
//! it), the meter is what they ask; one file each keeps both small.
use std::cell::RefCell;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::spendcap::{Ask, Say, TaskSpend};

/// Dollars a task may spend before it asks, when nothing says otherwise.
pub(crate) const DEFAULT_USD: f64 = 10.0;
/// Names the cap in dollars; the app sets it on a broker it spawns from the
/// settings form's `task_budget_usd`. `0` turns the ask off.
pub(crate) const ENV: &str = "CREW_TASK_BUDGET_USD";

/// The cap in micro-USD an environment value means: unset or unreadable is
/// the default, zero or less is off (`0`).
pub(crate) fn parse(raw: Option<&str>) -> u64 {
    let usd = raw
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(DEFAULT_USD);
    (usd.max(0.0) * 1e6).round() as u64
}

/// Micro-USD as the dollars a person reads: `$10`, `$10.40`.
pub(crate) fn usd(micros: u64) -> String {
    let cents = (micros + 5_000) / 10_000;
    match cents % 100 {
        0 => format!("${}", cents / 100),
        c => format!("${}.{c:02}", cents / 100),
    }
}

thread_local! {
    static CURRENT: RefCell<Option<Arc<TaskSpend>>> = const { RefCell::new(None) };
}

/// Restores the scope that was there before, when the task's scope ends.
pub(crate) struct Scope(Option<Arc<TaskSpend>>);

impl Drop for Scope {
    fn drop(&mut self) {
        let before = self.0.take();
        CURRENT.with(|c| *c.borrow_mut() = before);
    }
}

/// Make `task` this thread's spend meter until the scope drops.
pub(crate) fn enter(task: Option<Arc<TaskSpend>>) -> Scope {
    Scope(CURRENT.with(|c| std::mem::replace(&mut *c.borrow_mut(), task)))
}

/// The meter of the task running on this thread, if its cap is on.
pub(crate) fn current() -> Option<Arc<TaskSpend>> {
    CURRENT.with(|c| c.borrow().clone())
}

/// This task's meter as the pane's own events carry it: the cap from the
/// environment, the question as an approval the pane draws, the stop line as
/// a message. `None` when the cap is off.
pub(crate) fn live(
    cancel: Arc<AtomicBool>,
    emit: Arc<dyn Fn(crate::PluginEvent) + Send + Sync>,
) -> Option<Arc<TaskSpend>> {
    let cap = parse(std::env::var(ENV).ok().as_deref());
    let (asker, teller) = (Arc::clone(&emit), emit);
    // Whoever the task is for (`approval::Requester`): a trigger has nobody
    // awake to ask, so it stops at the cap rather than wait out the clock.
    let who = super::approval::Requester::from_env();
    let ask: Ask = Arc::new(move |id, question| {
        let reply_to = who.reply_to();
        if reply_to.is_empty() {
            return false;
        }
        asker(crate::PluginEvent::Approval {
            id: id.into(),
            tool: "spend cap".into(),
            tier: "read".into(),
            reply_to: reply_to.into(),
            question: question.into(),
        });
        true
    });
    let say: Say = Arc::new(move |line| teller(super::relay::msg("agent smith", line.to_string())));
    TaskSpend::new(cap, cancel, ask, say)
}

#[cfg(test)]
#[path = "spendscope_tests.rs"]
mod tests;
