//! The clock: firing what crew is waiting to do (goal:
//! docs/superpowers/goals/2026-09-01-close-the-open-goals.md, Pillar 1).
//!
//! The record and its arithmetic are [`super::intent`]; the storage is [`super::intentlog`];
//! this is the half that happens on the daemon's own loop, four times a second, with nobody
//! watching. That is why both of its rules are about restraint rather than capability.
use super::intent;
use super::watchchat::{self, Ask};

impl super::Daemon {
    /// Answer a watch command that arrived over a channel, or `None` when the message was not
    /// one and belongs to an agent.
    ///
    /// Reached BEFORE the message goes anywhere near a session, with one exception that matters:
    /// a conversation already blocked on an approval is answering a question, so nothing it says
    /// is read as a command. "cancel" means no, not `cancel w1`.
    pub(crate) fn watch_chat(&mut self, from: &str, said: &str, now_ms: u64) -> Option<String> {
        if self.bridge.is_awaiting(from) {
            return None;
        }
        Some(match watchchat::read(said, now_ms)? {
            Err(why) => why,
            Ok(Ask::List) => {
                let live = self.watch.live();
                if live.is_empty() {
                    "I am not watching for anything".to_string()
                } else {
                    live.iter()
                        .map(|i| {
                            format!(
                                "{}  {}  {}  {}",
                                i.id,
                                intent::until(i.fire_ms, now_ms),
                                i.repeat.label(),
                                i.text
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }
            Ok(Ask::Cancel(id)) => match self.watch.cancel(&id, now_ms) {
                Ok(true) => format!("{id} cancelled"),
                Ok(false) => format!("I am not watching for {id}"),
                Err(e) => format!("could not write the watchlist: {e}"),
            },
            Ok(Ask::Register {
                text,
                fire_ms,
                repeat,
            }) => match self.watch.add(&text, from, fire_ms, repeat, now_ms) {
                Ok(i) => format!("{}  {}  {}", i.id, intent::until(i.fire_ms, now_ms), i.text),
                Err(e) => format!("could not write the watchlist: {e}"),
            },
        })
    }

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
