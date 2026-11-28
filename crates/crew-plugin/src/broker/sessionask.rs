//! Putting one approval to the person at the pane. Moved out of `session.rs` whole, and
//! unchanged, to pay for the tool hooks (`hooks`) that went into `SessionTools::call`: that
//! file is far past the line cap and may not grow. A child of `session`, so the struct's
//! private fields stay in reach.
use std::sync::atomic::Ordering;

use super::SessionTools;
use crate::broker::hunkpick::{self, Picked};

/// The few commands even auto-approve asks about, and why (`danger`).
pub(super) fn danger(name: &str, args: &str) -> Option<&'static str> {
    let v = serde_json::from_str::<serde_json::Value>(args).ok()?;
    (name == "sys:run")
        .then(|| v.get("cmd")?.as_str().and_then(crate::broker::danger::of))
        .flatten()
}

impl SessionTools {
    /// Put approval `id` for `name` to the person at the pane and wait for their answer.
    /// `Ok` = they allowed it, whole or (`Some`) only the hunks they kept of a file change.
    /// `Err(None)` = nobody here can be asked; `Err(Some(why))` = they refused, did not answer
    /// in time, or stopped the task — `why` is told to the model.
    pub(super) fn ask(
        &self,
        id: &str,
        name: &str,
        args: &str,
        danger: Option<&str>,
    ) -> Result<Option<Picked>, Option<String>> {
        use crate::broker::approval as a;
        let pending = self
            .gate
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pending()
            .iter()
            .find(|p| p.id == id)
            .cloned();
        // The change a file edit would make, shown in the question so it is judged, not trusted.
        let shown = hunkpick::preview(name, args);
        let more = shown.as_ref().map(|s| s.more()).unwrap_or_default();
        let asked = self.requester.is_present_human()
            && pending.is_some_and(|p| {
                a::ask_host_about(&p, &a::what(name, args), danger, (&more.0, &more.1))
            });
        if !asked {
            return Err(None);
        }
        let answer = a::wait_for_answer_or(id, self.policy.timeout_ms, || {
            self.cancel.load(Ordering::Relaxed)
        });
        let kept = hunkpick::take(id);
        self.gate
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .answer(id, answer == Some(true));
        match answer {
            Some(true) => hunkpick::choose(shown, kept).map_err(Some),
            Some(false) => Err(Some(format!(
                "the user refused {name}, so it did not run — do not try it again; say what you \
                 wanted it for and ask how they would like to proceed"
            ))),
            None => Err(Some(format!(
                "{name} did not run: nobody approved it (no answer in {} minutes, or the task \
                 was stopped)",
                self.policy.timeout_ms / 60_000
            ))),
        }
    }

    /// A `sys` tool call, or — when the person kept only some hunks of a file change — that
    /// change in part.
    pub(super) fn run_sys(
        &self,
        kept: &Option<Picked>,
        tool: &str,
        args: &str,
    ) -> Result<String, String> {
        match kept {
            Some(k) => k.run(),
            None => crate::broker::systools::call(tool, args),
        }
    }
}
