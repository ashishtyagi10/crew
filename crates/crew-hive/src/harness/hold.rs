//! A turn the user's stop check holds open (`Tools::stop`, the Stop hook).
//!
//! When a reply asks for no tool it is the answer — unless the surface's stop
//! check says "not yet". Then the reason goes back to the model as a note of
//! its own and the turn goes on, the way Claude Code's Stop hook works. Two
//! things keep a check from holding a turn forever: a cap on how often, and
//! the end of holding once the answer stops changing, since a model that
//! answered the same thing twice has nothing more to give the reason. Either
//! way the answer stands and a failed `hook:stop` row says why, as does a
//! check that broke; a check that let the answer through says nothing.

use std::sync::Arc;
use std::time::Instant;

use super::TurnContext;
use crate::bus::HiveEvent;
use crate::tools::{StopCheck, Tools};

/// Most times one turn is held open. Each is a whole extra model call, and a
/// rule that three tries could not satisfy is one for the person to read.
pub(super) const MAX_HOLDS: u32 = 3;

/// The tool lane's name for the check, so the pane draws it as a tool row.
pub(super) const LABEL: &str = "hook:stop";

/// How often this turn has been held, and the answer it was held on last.
#[derive(Default)]
pub(super) struct Hold {
    times: u32,
    last: Option<String>,
}

impl Hold {
    /// The note to go on with when the check holds `answer`, or `None` to end.
    pub(super) async fn check(
        &mut self,
        ctx: &TurnContext,
        tools: Option<&Arc<dyn Tools>>,
        answer: &str,
    ) -> Option<String> {
        let tools = Arc::clone(tools?);
        if ctx.stopped() {
            return None;
        }
        let (text, held, began) = (answer.to_owned(), self.times > 0, Instant::now());
        // A hook runs for seconds: off the async thread, as every tool call is.
        let verdict = tokio::task::spawn_blocking(move || tools.stop(&text, held)).await;
        let ms = began.elapsed().as_millis() as u64;
        let reason = match verdict.unwrap_or(StopCheck::End) {
            StopCheck::End => return None,
            StopCheck::Broke(why) => {
                row(ctx, false, why, ms);
                return None;
            }
            StopCheck::Hold(reason) => reason,
        };
        let same = self.last.as_deref().map(str::trim) == Some(answer.trim());
        if same || self.times >= MAX_HOLDS {
            let why = match same {
                true => "the answer did not change".to_string(),
                false => format!("held {MAX_HOLDS} times already"),
            };
            row(ctx, false, format!("{why}, so it ends here: {reason}"), ms);
            return None;
        }
        self.times += 1;
        self.last = Some(answer.to_owned());
        let shown = format!("held the turn ({} of {MAX_HOLDS}): {reason}", self.times);
        row(ctx, true, shown, ms);
        Some(format!(
            "A stop hook of the user's held your answer back: {reason}\n\
             Do what it asks, then give your final answer again."
        ))
    }
}

/// The check as a call and its result on the tool lane, so the pane shows it
/// where it shows every other action: `ok` false is the failed (`!`) row.
fn row(ctx: &TurnContext, ok: bool, text: String, ms: u64) {
    let label = LABEL.to_string();
    ctx.publish(HiveEvent::ToolCall {
        agent: ctx.agent.clone(),
        label: label.clone(),
        args: "{}".into(),
    });
    ctx.publish(HiveEvent::ToolResult {
        agent: ctx.agent.clone(),
        label,
        ok,
        text,
        ms,
    });
}

#[cfg(test)]
#[path = "holdkit_tests.rs"]
mod kit;
#[cfg(test)]
#[path = "hold_tests.rs"]
mod tests;
