//! Prompt hooks: Claude Code's `UserPromptSubmit`, run once when a task reaches the broker's
//! worker, before any routing or model call, on the same contract as the other hooks. The hook
//! reads `{"event": "prompt", "prompt": …, "cwd": …}` on one line. Exit 2, or
//! `{"decision": "block", "reason": …}` on stdout, refuses the task: the pane is told
//! `blocked by hook: <reason>` and no model is dialled. Exit 0 with other stdout adds that text
//! to what the model is given this turn (`with_memory`, under `From your prompt hook:`).
//!
//! A hook that cannot run is `on_failure`'s call, as for a tool hook: `allow` logs it and runs
//! the task, `block` refuses it. A `hooks.json` that will not load refuses the task as it
//! refuses a tool call, naming the way out.
use std::cell::RefCell;

use super::run::{self, Ran};
use super::{blocked, cap, log, Hooks, OnFailure, Source};

thread_local! {
    static CONTEXT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Restores the context that was there before, when the task's scope ends.
pub(crate) struct Scope(Option<String>);

impl Drop for Scope {
    fn drop(&mut self) {
        CONTEXT.with(|c| *c.borrow_mut() = self.0.take());
    }
}

/// Make `text` this thread's prompt-hook context until the scope drops. The worker thread is
/// the task, and `with_memory` runs on it before any call fans out.
pub(crate) fn enter(text: Option<String>) -> Scope {
    Scope(CONTEXT.with(|c| c.replace(text)))
}

/// The running task's prompt-hook context, as the block the model is shown.
pub(crate) fn block() -> Option<String> {
    CONTEXT.with(|c| {
        c.borrow()
            .clone()
            .map(|t| format!("From your prompt hook:\n{t}"))
    })
}

/// What the user's prompt hooks make of `text`: `Err` is the one line the pane shows in place
/// of a run, `Ok` the context to add, if any. `None` is no hooks (`CREW_HOOKS=0`).
pub(crate) fn gate(src: Option<&Source>, text: &str) -> Result<Option<String>, String> {
    src.map_or(Ok(None), |s| s.load().prompt(text))
}

impl Hooks {
    pub(crate) fn prompt(&self, text: &str) -> Result<Option<String>, String> {
        let file = self.file.as_ref().map_err(|e| {
            format!("blocked by hook: {e} — fix the file, or set CREW_HOOKS=0 to turn hooks off")
        })?;
        let cwd = self.cwd();
        let line = serde_json::json!({
            "event": "prompt", "prompt": text, "cwd": cwd.to_string_lossy(),
        });
        let line = format!("{line}\n");
        let mut context = Vec::new();
        for rule in &file.prompt {
            match run::run(&rule.cmd, &cwd, &line, self.deadline) {
                Ran::Exited(code, out, err) => {
                    if let Some(why) = blocked(code, &out, &err) {
                        return Err(format!("blocked by hook: {why}"));
                    }
                    if code != 0 {
                        let why = err.lines().next().unwrap_or("").trim();
                        log(format!(
                            "prompt hook `{}` exited {code}, let through: {why}",
                            rule.cmd
                        ));
                    } else if !out.trim().is_empty() {
                        context.push(cap(out.trim()));
                    }
                }
                Ran::Failed(why) if rule.on_failure == OnFailure::Block => {
                    return Err(format!(
                        "blocked by hook: `{}` {why}, and it is set to block when it cannot run",
                        rule.cmd
                    ));
                }
                Ran::Failed(why) => log(format!("prompt hook `{}` {why}, let through", rule.cmd)),
            }
        }
        Ok((!context.is_empty()).then(|| context.join("\n")))
    }
}

#[cfg(all(test, unix))]
#[path = "hookprompt_tests.rs"]
mod tests;
