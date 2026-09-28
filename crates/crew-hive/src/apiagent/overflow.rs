//! A worker's request grown past the model's context, cut down and sent once
//! more.
//!
//! The native loop resends every turn each round, every tool result whole, so
//! a worker that reads a few big files reaches qwen-max's 32K tokens and the
//! provider refuses the call (`Range of input length should be [1, 30720]`).
//! The task ended on that error, failed, and everything it had read went
//! with it. Now that one error is answered by cutting the request and trying
//! once more: every result but the last round's becomes a line naming the
//! call, and the model can make the call again if it needs one back. From
//! then on the task stays cut, each older round stubbed as the next arrives,
//! and a second refusal ends it as before. The text loop cuts its log the
//! same way (`Exchanges::tighten`).
//!
//! The cost: what was cut is gone from the model's view, so a read it still
//! needed is a round spent reading it again.

use super::stubs::{shrink, size, stubbed};
use crate::agent::AgentContext;
use crate::bus::HiveEvent;
use crate::provider::{ProviderError, ToolInvocation, Turn};
use crate::tools::exchanges::{Exchanges, CONTEXT_FULL};
use crate::tools::seen::Seen;

/// Say the cut in the worker's live card: a delta, which settles away when
/// the answer lands, so it is shown while the retry runs and not kept as the
/// worker's words.
pub(super) fn say(ctx: &AgentContext) {
    ctx.bus.publish(HiveEvent::OutputDelta {
        agent: ctx.agent.clone(),
        text: format!("\n{CONTEXT_FULL}\n"),
    });
}

/// The text loop's one retry: `err` is the context, and the log had older
/// rounds to cut (said in the pane when so). `false` sends the error on.
pub(super) fn tightened(ctx: &AgentContext, err: &ProviderError, log: &mut Exchanges) -> bool {
    let again = err.is_context_overflow() && log.tighten();
    if again {
        say(ctx);
    }
    again
}

/// A native task's one cut: whether it has been made, and what it must come
/// in under.
pub(super) struct Cut<'a> {
    label: &'a (dyn Fn(&ToolInvocation) -> String + Sync),
    /// The prompt and system, the same in every request.
    fixed: usize,
    /// The largest request answered so far.
    fit: usize,
    /// The size of the request out now.
    out: usize,
    made: bool,
}

impl<'a> Cut<'a> {
    pub(super) fn new(
        prompt: &str,
        system: Option<&str>,
        label: &'a (dyn Fn(&ToolInvocation) -> String + Sync),
    ) -> Self {
        Self {
            label,
            fixed: prompt.len() + system.map_or(0, str::len),
            fit: 0,
            out: 0,
            made: false,
        }
    }

    /// A request carrying `turns` is going out.
    pub(super) fn sending(&mut self, turns: &[Turn]) {
        self.out = size(self.fixed, turns);
    }

    /// It was answered: a request that size fits.
    pub(super) fn answered(&mut self) {
        self.fit = self.fit.max(self.out);
    }

    /// It was refused with `err`. When that is the context, the first time,
    /// and there is something to cut: cut `turns`, forget every read (see
    /// [`Cut::after_round`]), say so, and `true` to send it again.
    pub(super) fn refused(
        &mut self,
        ctx: &AgentContext,
        err: &ProviderError,
        turns: &mut Vec<Turn>,
        seen: &mut Seen,
    ) -> bool {
        if self.made
            || !err.is_context_overflow()
            || !shrink(turns, self.fixed, self.fit, self.label)
        {
            return false;
        }
        self.made = true;
        *seen = Seen::default();
        say(ctx);
        true
    }

    /// A round was added to a cut task. The round before it becomes stubs,
    /// and every read is forgotten: the next reply's calls go into a request
    /// where only their own round is whole, so a pointer at any earlier one
    /// would name a result the model can no longer see; forgotten, a repeat
    /// runs again.
    pub(super) fn after_round(&self, turns: &mut Vec<Turn>, seen: &mut Seen) {
        if self.made {
            *turns = stubbed(turns, self.label);
            *seen = Seen::default();
        }
    }
}

#[cfg(test)]
#[path = "overflowkit_tests.rs"]
mod kit;
#[cfg(test)]
#[path = "overflow_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "overflowtext_tests.rs"]
mod text_tests;
