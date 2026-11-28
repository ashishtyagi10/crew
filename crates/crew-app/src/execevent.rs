//! `crew exec`: what ONE broker event means for a headless run. Pure — no I/O, no clock — so
//! the mapping that decides the exit code is unit-tested without a broker.
use crew_plugin::{PluginCommand, PluginEvent};

/// Exit codes of `crew exec`, named so the docs, the help and the tests say the same thing.
pub(crate) const EXIT_OK: i32 = 0;
pub(crate) const EXIT_FAILED: i32 = 1;
pub(crate) const EXIT_USAGE: i32 = 2;
pub(crate) const EXIT_REFUSED: i32 = 3;

/// What the driver should do after one event: lines for stderr, and maybe one command to send.
#[derive(Default, Debug)]
pub(crate) struct Reaction {
    pub log: Vec<String>,
    pub reply: Option<PluginCommand>,
}

/// The run so far. The answer is the LAST reply the task produced (a swarm's merge step speaks
/// last), because stdout must hold exactly one thing a script can use.
pub(crate) struct Run {
    /// `--yes`: answer approvals with allow instead of deny.
    pub allow: bool,
    pub answer: Option<String>,
    /// An approval was asked and (without `--yes`) refused: exit 3, not a mystery failure.
    pub refused: bool,
    pub errors: Vec<String>,
    task: Option<u64>,
    pub done: bool,
}

impl Run {
    pub fn new(allow: bool) -> Self {
        Run {
            allow,
            answer: None,
            refused: false,
            errors: Vec::new(),
            task: None,
            done: false,
        }
    }

    /// Fold one event in.
    pub fn absorb(&mut self, ev: PluginEvent) -> Reaction {
        let mut r = Reaction::default();
        match ev {
            // Ours is the first task the broker announces; its end is the run's end.
            PluginEvent::Task { id, running, .. } => {
                if running && self.task.is_none() {
                    self.task = Some(id);
                } else if !running && self.task == Some(id) {
                    self.done = true;
                }
            }
            PluginEvent::Message {
                sender, text, meta, ..
            } if meta.starts_with("task:") => {
                if sender == "agent smith" {
                    r.log.push(text); // routing / plan notes: progress, not the answer
                } else {
                    r.log.push(format!("[{sender}] {}", first_line(&text)));
                    self.answer = Some(text);
                }
            }
            PluginEvent::Hive {
                event: crew_hive::HiveEvent::ToolCall { label, args, .. },
            } => r.log.push(format!("tool {label} {}", clip(&args, 100))),
            PluginEvent::Status { message, .. } => r.log.push(message),
            PluginEvent::Error { message } => {
                r.log.push(format!("error: {message}"));
                self.errors.push(message);
            }
            PluginEvent::Approval {
                id, tool, question, ..
            } => {
                if !self.allow {
                    self.refused = true;
                }
                let verdict = if self.allow {
                    "allowed"
                } else {
                    "refused (pass --yes)"
                };
                r.log.push(format!(
                    "approval {verdict}: {tool}: {}",
                    first_line(&question)
                ));
                r.reply = Some(PluginCommand::Approve {
                    id,
                    granted: self.allow,
                    hunks: None,
                });
            }
            _ => {}
        }
        r
    }

    /// The exit code once the run is over. A refused approval outranks a failure: it is the
    /// cause, and the one a script can act on (re-run with `--yes`).
    pub fn exit_code(&self) -> i32 {
        if self.refused {
            EXIT_REFUSED
        } else if !self.errors.is_empty() || self.answer.is_none() {
            EXIT_FAILED
        } else {
            EXIT_OK
        }
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or("")
}

fn clip(s: &str, max: usize) -> String {
    let one = s.replace('\n', " ");
    match one.char_indices().nth(max) {
        Some((i, _)) => format!("{}\u{2026}", &one[..i]),
        None => one,
    }
}

#[cfg(test)]
#[path = "execevent_tests.rs"]
mod tests;
