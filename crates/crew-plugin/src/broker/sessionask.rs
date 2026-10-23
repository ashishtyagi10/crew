//! Putting one approval to the person at the pane. Moved out of `session.rs` whole, and
//! unchanged, to pay for the tool hooks (`hooks`) that went into `SessionTools::call`: that
//! file is far past the line cap and may not grow. A child of `session`, so the struct's
//! private fields stay in reach.
use std::sync::atomic::Ordering;

use super::SessionTools;

impl SessionTools {
    /// Put approval `id` for `name` to the person at the pane and wait for their answer.
    /// `Ok` = they allowed it. `Err(None)` = nobody here can be asked; `Err(Some(why))` = they
    /// refused, did not answer in time, or stopped the task — `why` is told to the model.
    pub(super) fn ask(
        &self,
        id: &str,
        name: &str,
        args: &str,
        danger: Option<&str>,
    ) -> Result<(), Option<String>> {
        use crate::broker::approval as a;
        let pending = self
            .gate
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .pending()
            .iter()
            .find(|p| p.id == id)
            .cloned();
        let asked = self.requester.is_present_human()
            && pending.is_some_and(|p| a::ask_host_about(&p, &a::what(name, args), danger));
        if !asked {
            return Err(None);
        }
        let answer = a::wait_for_answer_or(id, self.policy.timeout_ms, || {
            self.cancel.load(Ordering::Relaxed)
        });
        self.gate
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .answer(id, answer == Some(true));
        match answer {
            Some(true) => Ok(()),
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
}
