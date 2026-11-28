//! One [`TaskSpend`] is the TASK's, entered on the task's worker thread
//! (`spendscope::enter`, beside `cancelscope`): every agent of a swarm and
//! every hop of a relay reads and feeds the same meter, so the cap is per
//! task, never per agent. It is checked before each model call
//! ([`TaskSpend::before_call`]) — between tool rounds and hops, never in the
//! middle of a stream.
//!
//! Where it is held (each reads the scope on the TASK thread, then carries
//! the meter to wherever the call runs):
//! - a relay hop's provider and a swarm's planner and workers
//!   (`spendcapwrap::wrap`, `wrap_swarm`): asked from inside the call, which
//!   no bound surrounds;
//! - the swarm's closing answer, the swarm's judge and the reply judge
//!   (`swarmanswer`, `swarmverify`, `replyverify`) and an `@a+b` fan-out
//!   (`fan_out`): a [`Gate`](super::spendgate::Gate) asks BEFORE the call and
//!   charges after, because a 30 s or 3 min bound around a question that may
//!   wait five minutes would time out mid-question. The fan's agents were
//!   built at startup, before any task, so there is no provider to wrap.
//!
//! What counts: the cost of each completed call, as the provider reported it,
//! else the price list's estimate (`pricing::estimate`). A model the list has
//! never heard of counts 0, so a tokens-only provider cannot reach the cap;
//! better a missing stop than a made-up price. A stream cut off by `/stop`
//! never completes, so its partial cost is not counted: the cap is a floor
//! under what was spent, not a bill. A fan agent's own tool rounds are one
//! call to the gate, so a fan can overshoot by an agent's whole turn.
//!
//! Not held, on purpose: the advisor (a second-opinion model whose cost
//! already joins the thread's sums, `advise`) and `sys:explore` (a read-only
//! sub-agent with limits and a bill of its own, `sysexplore`) dial providers
//! of their own and are not counted; neither is the cheap routing,
//! classifier and summary traffic, which costs cents and must still answer
//! at the cap so the task can say why it stopped.
//!
//! A question nobody answers within the approval clock (five minutes, as a
//! tool approval) is a no: money is not spent on a yes nobody gave. A task
//! with nobody awake to ask (a trigger) stops at the cap at once.
//!
//! A swarm's first ask comes at $1, not at the cap: the old silent $1
//! governor killed a runaway swarm, asking at the same point lets a good one
//! go on ([`TaskSpend::first_ask_at`]); a yes then raises by the usual step.
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) use super::spendscope::{current, enter, live, usd};

/// Puts the question to whoever can answer; `false` when nobody can be asked.
pub(crate) type Ask = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;
/// Says one line in the task's pane.
pub(crate) type Say = Arc<dyn Fn(&str) + Send + Sync>;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// One task's meter, its cap, and the one question it may be waiting on.
pub(crate) struct TaskSpend {
    spent: AtomicU64,
    cap: AtomicU64,
    /// What a yes adds: the cap the task began with.
    step: u64,
    /// The task's stop. Set by `/stop` and, on a no, by this.
    cancel: Arc<AtomicBool>,
    ended: AtomicBool,
    /// Held while a question is open, so agents that cross the cap together
    /// ask once and the rest wait on the same answer.
    asking: tokio::sync::Mutex<()>,
    ask: Ask,
    say: Say,
    wait: Duration,
}

impl TaskSpend {
    /// A meter for a task that may spend `cap` micro-USD before asking; `None`
    /// when the cap is off.
    pub(crate) fn new(cap: u64, cancel: Arc<AtomicBool>, ask: Ask, say: Say) -> Option<Arc<Self>> {
        (cap > 0).then(|| {
            Arc::new(Self {
                spent: AtomicU64::new(0),
                cap: AtomicU64::new(cap),
                step: cap,
                cancel,
                ended: AtomicBool::new(false),
                asking: tokio::sync::Mutex::new(()),
                ask,
                say,
                wait: Duration::from_millis(super::approval::DEFAULT_TIMEOUT_MS),
            })
        })
    }

    /// How long an unanswered question stays open (a test shortens it).
    #[cfg(test)]
    pub(crate) fn waiting(mut self: Arc<Self>, wait: Duration) -> Arc<Self> {
        Arc::get_mut(&mut self).expect("not shared yet").wait = wait;
        self
    }

    /// Make the FIRST question come at `micros` when that is below the cap
    /// the task began with (a swarm's $1). A no-op once the cap has moved, so
    /// a second swarm of the same task cannot undo a yes.
    pub(crate) fn first_ask_at(&self, micros: u64) {
        let _ = self.cap.compare_exchange(
            self.step,
            self.step.min(micros),
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
    }

    /// A completed call's cost joins the task's.
    pub(crate) fn add(&self, micros: u64) {
        self.spent.fetch_add(micros, Ordering::Relaxed);
    }

    /// Micro-USD the task has spent.
    pub(crate) fn spent(&self) -> u64 {
        self.spent.load(Ordering::Relaxed)
    }

    pub(crate) fn over(&self) -> bool {
        self.spent() >= self.cap.load(Ordering::Relaxed)
    }

    /// Before a model call: `Ok` to go on, `Err` when the task is over its
    /// cap and was not allowed more (or was stopped while it waited).
    ///
    /// Asks once however many agents arrive together, and waits no longer
    /// than an approval does. Silence is no: money is not spent on a yes
    /// nobody gave. The wait polls the task's stop, so `/stop` ends it.
    pub(crate) async fn before_call(&self) -> Result<(), String> {
        const GONE: &str = "stopped at the spend cap";
        if !self.over() {
            return Ok(());
        }
        let _one = self.asking.lock().await;
        loop {
            if self.ended.load(Ordering::Relaxed) {
                return Err(GONE.into());
            }
            if self.cancel.load(Ordering::Relaxed) {
                return Err("stopped by you".into());
            }
            if !self.over() {
                return Ok(());
            }
            match self.put().await {
                Some(true) => {
                    self.cap.fetch_add(self.step, Ordering::Relaxed);
                }
                Some(false) | None if self.cancel.load(Ordering::Relaxed) => {}
                Some(false) | None => {
                    self.ended.store(true, Ordering::Relaxed);
                    (self.say)(&format!("stopped at the spend cap ({})", usd(self.spent())));
                    self.cancel.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    /// The question, and its answer: `None` when nobody could be asked, nobody
    /// answered in time, or the task was stopped meanwhile.
    async fn put(&self) -> Option<bool> {
        let id = format!("cap{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let q = format!(
            "spent {} of the {} cap \u{2014} Enter/y continues (+{}) \u{b7} Esc/n stops",
            usd(self.spent()),
            usd(self.cap.load(Ordering::Relaxed)),
            usd(self.step)
        );
        if !(self.ask)(&id, &q) {
            return None;
        }
        let began = Instant::now();
        while began.elapsed() < self.wait && !self.cancel.load(Ordering::Relaxed) {
            if let Some(a) = super::approval::take_answer(&id) {
                return Some(a);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        None
    }
}

#[cfg(test)]
#[path = "spendcover_tests.rs"]
mod cover_tests;
#[cfg(test)]
#[path = "spendkit_tests.rs"]
pub(crate) mod kit;
#[cfg(test)]
#[path = "spendcap_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "spendwalk_tests.rs"]
mod walk_tests;
