//! A per-task spend cap: past it the task pauses and asks, it never runs on.
//!
//! WHY: crew showed what a task cost (`$0.214` on the status line, the
//! receipt) but nothing stopped a looping agent or a wide swarm from spending
//! without limit overnight. Claude Code's `--max-budget-usd` and Aider's cost
//! line are the same answer; this one asks first, since a task past $10 may be
//! worth $20.
//!
//! One [`TaskSpend`] is the TASK's, entered on the task's worker thread
//! ([`enter`], beside `cancelscope`): every agent of a swarm and every hop of
//! a relay reads and feeds the same meter, so the cap is per task, never per
//! agent. It is checked before each model call ([`TaskSpend::before_call`],
//! from `spendcapwrap::Capped`) — between tool rounds and hops, never in the
//! middle of a stream.
//!
//! What counts: the cost of each completed call, as the provider reported it,
//! else the price list's estimate (`pricing::estimate`). A model the list has
//! never heard of counts 0, so a tokens-only provider cannot reach the cap;
//! better a missing stop than a made-up price. The advisor's and `sys:explore`'s
//! own calls go through their own providers and are not counted.
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Puts the question to whoever can answer; `false` when nobody can be asked.
pub(crate) type Ask = Arc<dyn Fn(&str, &str) -> bool + Send + Sync>;
/// Says one line in the task's pane.
pub(crate) type Say = Arc<dyn Fn(&str) + Send + Sync>;

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

    /// A completed call's cost joins the task's.
    pub(crate) fn add(&self, micros: u64) {
        self.spent.fetch_add(micros, Ordering::Relaxed);
    }

    /// Micro-USD the task has spent.
    pub(crate) fn spent(&self) -> u64 {
        self.spent.load(Ordering::Relaxed)
    }

    fn over(&self) -> bool {
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
            "spent {} of the {} cap \u{2014} continue? Enter or y raises it by {}; Esc or n stops",
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
#[path = "spendcap_tests.rs"]
mod tests;
