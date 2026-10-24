//! A second model reads a failed check, and the agent is told what it made
//! of it — on either engine, as the turn's [`Advice`].
//!
//! WHY: a weaker executor misreads a failure. Live, qwen-max answered a
//! `cargo build` that failed on one type error by reading files again, the
//! error itself unread. Anthropic's advisor tool, Claude Code's advisor, Amp's
//! oracle and Codex's Guardian answer that moment the same way: a second
//! model that only reads looks at the failure, and what it says goes to the
//! agent as context. The agent decides; nothing here runs a tool, edits the
//! request or changes what the turn does next.
//!
//! When: a tool round in which a `sys:run` ran and exited non-zero
//! (`Tools::failed`), at most [`PER_TURN`] times a turn. It reads the request,
//! the turn's calls one line each, and the failure cut to its useful part
//! (`failexcerpt`), which keeps first the line a run cut for length opens
//! with, every error by its place: the errors the cut dropped are the ones
//! the agent could not see. It writes three short lines at most. They join
//! the turn after the round's results, under [`HEAD`], until a later
//! failure's advice replaces them, and the engine says so in one quiet line
//! ([`Hooks::say`]). An advisor that errors or times out adds nothing and
//! says nothing: the turn is what it would have been.
//!
//! The relay's turn had it first (crew-plugin `advise`, which now only says
//! where the advisor's model comes from); a swarm worker has it since the
//! advisor moved here, so its failed build is read too.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::harness::Advice;
use crate::spent::Spent;
use crate::tools::{ToolCall, Tools};

mod text;

pub use text::HEAD;

/// Most advisor calls in one turn. A turn that keeps failing past two reads
/// has a problem a third read of the same output will not name.
pub const PER_TURN: u32 = 2;

/// What one advisor call comes back with: its reply and what it cost, or why
/// it did not answer.
pub type Asked = Pin<Box<dyn Future<Output = Result<(String, Spent), String>> + Send>>;

/// One advisor call: its brief in.
pub type Ask = Arc<dyn Fn(String) -> Asked + Send + Sync>;

/// Where a turn's advisor call comes from, looked up once a check has failed
/// (finding a provider can read a credential store), on the turn's own
/// thread: `None` when none may run.
pub type Find = Arc<dyn Fn() -> Option<Ask> + Send + Sync>;

/// What the engine does with what the advisor did.
#[derive(Clone)]
pub struct Hooks {
    /// Its one quiet line when it gave advice: `advised · <first line>`.
    pub say: Arc<dyn Fn(&str) + Send + Sync>,
    /// What a read cost, for the engine's sums.
    pub spent: Arc<dyn Fn(Spent) + Send + Sync>,
}

/// One turn's advisor.
pub struct Advisor {
    /// The request as the person made it, which the brief opens with.
    request: String,
    tools: Arc<dyn Tools>,
    find: Find,
    hooks: Hooks,
    state: Mutex<State>,
}

/// The call, once a failure needed it (`Some(None)` when none may run), how
/// often it was asked, the turn's calls so far, and the newest advice as its
/// section.
#[derive(Default)]
struct State {
    ask: Option<Option<Ask>>,
    asked: u32,
    calls: Vec<String>,
    advice: String,
}

impl Advisor {
    /// The advisor for a turn on `request`, whose calls `tools` judges, its
    /// model found by `find` when a check first fails.
    pub fn new(
        request: impl Into<String>,
        tools: Arc<dyn Tools>,
        find: Find,
        hooks: Hooks,
    ) -> Self {
        Self {
            request: request.into(),
            tools,
            find,
            hooks,
            state: Mutex::default(),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Note one round's `calls` and what each came back as, and when one was
    /// a failed `sys:run` and the turn has a read left: the call to make and
    /// its brief. `None` leaves the advice as it was.
    fn noted(&self, calls: &[ToolCall], ran: &[(bool, String)]) -> Option<(Ask, String)> {
        let mut s = self.state();
        for (call, (ok, _)) in calls.iter().zip(ran) {
            let n = s.calls.len() + 1;
            s.calls.push(text::call_line(n, call, *ok));
        }
        let failed = calls.iter().zip(ran).find(|(c, (_, out))| {
            (c.server.as_str(), c.tool.as_str()) == ("sys", "run")
                && self.tools.failed(&c.server, &c.tool, out)
        });
        let (call, (_, out)) = failed.filter(|_| s.asked < PER_TURN)?;
        let ask = s.ask.get_or_insert_with(|| (self.find)()).clone()?;
        s.asked += 1;
        Some((ask, text::brief(&self.request, &s.calls, call, out)))
    }
}

impl Advice for Advisor {
    fn read<'a>(
        &'a self,
        calls: &'a [ToolCall],
        ran: &'a [(bool, String)],
    ) -> Pin<Box<dyn Future<Output = String> + Send + 'a>> {
        Box::pin(async move {
            let Some((ask, brief)) = self.noted(calls, ran) else {
                return self.state().advice.clone();
            };
            let Ok((reply, spent)) = ask(brief).await else {
                return self.state().advice.clone();
            };
            (self.hooks.spent)(spent);
            let lines = text::advice(&reply);
            let Some(first) = lines.first() else {
                return self.state().advice.clone();
            };
            (self.hooks.say)(&text::said(first));
            let section = text::section(&lines);
            self.state().advice = section.clone();
            section
        })
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
