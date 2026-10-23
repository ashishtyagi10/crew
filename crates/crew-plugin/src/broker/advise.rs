//! A second model reads a failed check, and the agent is told what it made
//! of it.
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
//! with, every error by its place (`rundigest`): the errors the cut dropped
//! are the ones the agent could not see. It writes three short lines at most.
//! They join every follow-up after the exchanges, under `advicetext::HEAD`,
//! until a later failure's advice replaces them, and smith says so in one
//! quiet line. An advisor that errors or times out adds nothing and says
//! nothing: the follow-up is what it would have been.
//!
//! `CREW_ADVISOR=0` turns it off; `CREW_ADVISOR_MODEL` names the model, else
//! it is the provider's capable tier. On DashScope that is the chain head,
//! often the executor's own model: a fresh read of the error alone is the
//! point, not a bigger model.
use std::sync::Arc;

use crate::broker::intent::classify::{complete_once, ONESHOT_TIMEOUT};
use crate::broker::toolbatch::Ran;
use crate::broker::toolcall::ToolRunner;
use crate::broker::{Envelope, RunStats};
use crew_hive::tools::ToolCall;
use crew_hive::{ModelTier, Provider, Spent};

/// Most advisor calls in one turn. A turn that keeps failing past two reads
/// has a problem a third read of the same output will not name.
pub(crate) const PER_TURN: u32 = 2;

/// The tier the advisor runs on when `CREW_ADVISOR_MODEL` names none.
const TIER: ModelTier = ModelTier::Capable;

/// Output ceiling. The advice is three short lines, but Opus 5.5 always
/// thinks first, and its thinking is paid out of this.
const MAX_TOKENS: u32 = 1024;

/// One advisor call: its brief in, its reply and what it cost out.
pub(crate) type Ask = dyn Fn(&str) -> Result<(String, Spent), String>;

/// One turn's advisor: the call, once a failure needed it (`Some(None)` when
/// none may run), how often it was asked, the turn's calls so far, and the
/// newest advice as its follow-up section.
#[derive(Default)]
pub(crate) struct Advisor {
    ask: Option<Option<Box<Ask>>>,
    asked: u32,
    calls: Vec<String>,
    advice: String,
}

impl Advisor {
    /// Note one round's `calls` and what each came back as (`ran`), and when
    /// one was a failed `sys:run` and the turn has a read left, ask what the
    /// first of them means against the request in `env`. Its cost joins
    /// `stats`. Returns the section the follow-up carries: the newest advice,
    /// or empty.
    ///
    /// Not asked once the turn is `stopped`: the loop looks at the stop after
    /// this read, so a read for a stop pressed while the tools ran would be
    /// paid for and never sent.
    pub(crate) fn read(
        &mut self,
        runner: &dyn ToolRunner,
        env: &Envelope,
        calls: &[ToolCall],
        ran: &[Ran],
        stats: &mut RunStats,
        stopped: bool,
    ) -> String {
        for (call, r) in calls.iter().zip(ran) {
            let n = self.calls.len() + 1;
            self.calls.push(text::call_line(n, call, r.ok));
        }
        let failed = calls.iter().zip(ran).find(|(c, r)| {
            (c.server.as_str(), c.tool.as_str()) == ("sys", "run")
                && runner.failed(&c.server, &c.tool, &r.text)
        });
        let Some((call, r)) = failed.filter(|_| self.asked < PER_TURN && !stopped) else {
            return self.advice.clone();
        };
        let Some(ask) = self.ask.get_or_insert_with(live).as_deref() else {
            return self.advice.clone();
        };
        self.asked += 1;
        let Ok((reply, spent)) = ask(&text::brief(&env.body, &self.calls, call, &r.text)) else {
            return self.advice.clone();
        };
        stats.real_tokens += spent.tokens() as usize;
        stats.tok_in += spent.input;
        stats.tok_out += spent.output;
        stats.cost_microusd += spent.micros_usd;
        let lines = text::advice(&reply);
        if let Some(first) = lines.first() {
            crate::broker::steer::say(crate::broker::relay::msg("agent smith", text::said(first)));
            self.advice = text::section(&lines);
        }
        self.advice.clone()
    }
}

/// Whether `CREW_ADVISOR` (passed in) leaves the advisor on: anything but `0`.
fn on(knob: Option<&str>) -> bool {
    knob.map(str::trim) != Some("0")
}

/// The advisor a turn may ask, from the environment. Resolved only once a
/// check has failed: finding the provider reads the credential store.
#[cfg(not(test))]
fn live() -> Option<Box<Ask>> {
    let knob = std::env::var("CREW_ADVISOR").ok();
    let model = std::env::var("CREW_ADVISOR_MODEL").ok();
    let found = || crate::broker::discover::provider_and_model_for(TIER);
    resolve(knob.as_deref(), model, found)
}

/// Lib tests never reach a provider: a turn's advisor is the one its test put
/// on this thread, else none.
#[cfg(test)]
fn live() -> Option<Box<Ask>> {
    tests::installed()
}

/// The advisor on the provider `found` names, `model` overriding its model:
/// `None` when the knob is off (before anything is resolved), with no
/// provider, or under the mock. One plain completion: no tools, no system
/// prompt, nothing but the brief, at the one-shot bound the judges use.
fn resolve(
    knob: Option<&str>,
    model: Option<String>,
    found: impl FnOnce() -> Option<(Arc<dyn Provider>, String)>,
) -> Option<Box<Ask>> {
    if !on(knob) {
        return None;
    }
    let (provider, tier_model) = found().filter(|(_, m)| m != "mock")?;
    let model = model.filter(|m| !m.trim().is_empty()).unwrap_or(tier_model);
    Some(Box::new(move |brief: &str| {
        let c = complete_once(&provider, &model, brief, MAX_TOKENS, ONESHOT_TIMEOUT)?;
        Ok((c.text.clone(), Spent::billed(&model, TIER, &c)))
    }))
}

#[path = "advicetext.rs"]
mod text;

#[cfg(test)]
#[path = "advise_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "adviseround_tests.rs"]
mod round_tests;

#[cfg(test)]
#[path = "adviselive_tests.rs"]
mod live_tests;
