//! The clock: firing what crew is waiting to do (goal:
//! docs/superpowers/goals/2026-09-01-close-the-open-goals.md, Pillar 1).
//!
//! The record and its arithmetic are [`super::intent`]; the storage is [`super::intentlog`];
//! this is the half that happens on the daemon's own loop, four times a second, with nobody
//! watching. That is why both of its rules are about restraint rather than capability.
use super::intent;

impl super::Daemon {
    /// Fire everything whose time has come, and say so on the channel it answers on.
    ///
    /// The firing is RECORDED BEFORE the work is dispatched. A crash between the two costs one
    /// run; recording afterwards would cost the same crash an infinite loop, re-firing a
    /// past-due intent on every poll for as long as the daemon keeps dying.
    ///
    /// Each intent runs in a session of its own, opened as [`Requester::Trigger`] — the most
    /// restricted requester there is, and the reason a scheduled run can read the world but
    /// cannot do anything irreversible without a human in the loop.
    pub(crate) fn service_intents(&mut self, now_ms: u64) {
        let due: Vec<intent::Intent> = self
            .watch
            .live()
            .into_iter()
            .filter(|i| i.due(now_ms))
            .collect();
        for it in due {
            let skipped = match self.watch.record_fire(&it, now_ms) {
                Ok(n) => n,
                // An unwritable watchlist must not become a runaway: without the record there
                // is nothing to stop this firing again on the next poll.
                Err(e) => {
                    println!("could not record the firing of {}: {e}", it.id);
                    continue;
                }
            };
            let mut lines = vec![format!("{} \u{00b7} {}", it.id, it.text)];
            if let Some(note) = it.late_note(now_ms) {
                lines.push(note);
            }
            if skipped > 0 {
                lines.push(format!(
                    "({skipped} earlier firing(s) were missed and will not be run)"
                ));
            }
            let who = crew_plugin::approval::Requester::Trigger(it.id.clone());
            let key = format!("trigger:{}", it.id);
            if let Err(e) =
                self.bridge
                    .dispatch_as(&mut self.sessions, &key, &it.to, &who, &it.text)
            {
                lines.push(e);
            }
            if let Err(e) = self.channels.send(&it.to, &lines.join("\n")) {
                println!("could not announce {} to {}: {e}", it.id, it.to);
            }
        }
    }
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "clockwire_tests.rs"]
mod wire_tests;
