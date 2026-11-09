//! Stop hooks: the user's own "done?" check, run when an agent is about to end its turn —
//! Claude Code's `Stop` hook, under the same contract as the tool hooks. The hook reads one JSON
//! line, `{"event": "stop", "answer": …, "stop_hook_active": …, "cwd": …}`, and holds the turn
//! open with exit 2 (stderr the reason) or `{"decision": "block", "reason": …}` on stdout. The
//! reason goes back to the agent and the turn goes on; the harness caps how often (`hold`).
//!
//! `stop_hook_active` is true once this turn has been held already, Claude Code's flag for the
//! same purpose: a hook can let the second try through rather than hold for ever.
//!
//! A hook that cannot run, or runs past its deadline, does not hold the turn unless it is set
//! to (`on_failure: block`), the tool hooks' rule; it ends as a failed `hook:stop` row instead.
//! So does a `hooks.json` that will not load: a tool call it cannot vouch for is refused, but
//! an answer already written is the user's to read, and holding it would set the agent to
//! "fixing" a file it was never asked about.
use crew_hive::tools::StopCheck;

use super::run::{self, Ran};
use super::{blocked, log, Hooks, OnFailure};

impl Hooks {
    /// What the Stop hooks make of `answer`: the first that holds it wins.
    pub(crate) fn stop(&self, answer: &str, held: bool) -> StopCheck {
        let file = match &self.file {
            Ok(file) => file,
            Err(e) => return StopCheck::Broke(format!("stop hooks not run: {e}")),
        };
        let cwd = self.cwd();
        let line = serde_json::json!({
            "event": "stop",
            "answer": answer,
            "stop_hook_active": held,
            "cwd": cwd.to_string_lossy(),
        });
        let line = format!("{line}\n");
        let mut broke = None;
        for rule in &file.stop {
            let cmd = &rule.cmd;
            match run::run(cmd, &cwd, &line, self.deadline) {
                Ran::Exited(code, out, err) => match blocked(code, &out, &err) {
                    Some(why) => return StopCheck::Hold(format!("{why} (hook `{cmd}`)")),
                    // Only exit 2 holds, as only exit 2 blocks a call: anything else is a
                    // hook having trouble, a note for the person and nothing for the agent.
                    None if code != 0 => log(format!("stop hook `{cmd}` exited {code}, let end")),
                    None => {}
                },
                Ran::Failed(why) if rule.on_failure == OnFailure::Block => {
                    return StopCheck::Hold(format!(
                        "stop hook `{cmd}` {why}, and it is set to hold the turn when it cannot run"
                    ));
                }
                Ran::Failed(why) => {
                    broke.get_or_insert(format!("stop hook `{cmd}` {why}"));
                }
            }
        }
        broke.map_or(StopCheck::End, StopCheck::Broke)
    }
}

#[cfg(all(test, unix))]
#[path = "hookstop_tests.rs"]
mod tests;
