//! Where a swarm run goes when it ends: the live block folds into ONE
//! system-voice card in the transcript — the status line's final count over
//! the same task rows — so what the run did is still there to read after
//! the rows stop moving. A long plan folds like every other system card
//! (`chatfold`: three body lines, click to open), so the record costs the
//! transcript the header and a few lines until someone wants the rest.
//!
//! The pane's `HivePlan`/`Hive` intake lives here too, next to the fold it
//! ends in. The aggregate `Stats` lands AFTER the fold (the broker sums the
//! run once the scheduler returns), so the tool pool it carries is added to
//! the record in place rather than lost.
use crew_hive::{HiveEvent, TaskSpec, TaskState};

use crate::chat::ChatPane;
use crate::chatswarm::SwarmStatus;

/// The record's first words — how a later `Stats` finds the card to amend.
pub(crate) const LEAD: &str = "swarm \u{b7} ";
/// The record's sender: the run's lead, whose voice folds (`chatcard`).
const SENDER: &str = "agent smith";

/// `tools 5/12`, or nothing while the pool is unknown or unsized.
pub(crate) fn tools_words(tools: Option<(u32, u32)>) -> Option<String> {
    tools
        .filter(|(_, total)| *total > 0)
        .map(|(used, total)| format!("tools {used}/{total}"))
}

/// One task's row as plain text — the record's line, always whole.
pub(crate) fn plain(s: &SwarmStatus, i: usize) -> String {
    let (glyph, ..) = crate::swarm::view::state_style(s.tasks[i].state);
    let (spec, title, deps) = crate::chatswarmrows::words(s, i, usize::MAX, 0);
    let w = s.tasks.len().to_string().len();
    format!(" {:>w$} {glyph} {spec}{title}{deps}", i + 1)
}

/// The card's text at `now_ms`: `swarm · 3 tasks · 2 done · 1 failed · 12s
/// · tools 5/12`, then one line per task (`chatswarmrows::plain`).
pub(crate) fn text(s: &SwarmStatus, now_ms: u64) -> String {
    let count = |state: TaskState| s.tasks.iter().filter(|t| t.state == state).count();
    let mut head = vec![
        crate::wording::count(s.tasks.len(), "task"),
        format!("{} done", count(TaskState::Done)),
    ];
    for (state, word) in [
        (TaskState::Failed, "failed"),
        (TaskState::Cancelled, "cancelled"),
    ] {
        let n = count(state);
        if n > 0 {
            head.push(format!("{n} {word}"));
        }
    }
    if let Some((t0, t1)) = crate::chatswarmspan::axis(&s.tasks, now_ms) {
        head.push(crate::chatswarmspan::fmt_ms(t1 - t0));
    }
    head.extend(tools_words(s.tools));
    let rows: Vec<String> = (0..s.tasks.len()).map(|i| plain(s, i)).collect();
    format!("{LEAD}{}\n{}", head.join(" \u{b7} "), rows.join("\n"))
}

impl ChatPane {
    /// A swarm plan landed: open (or reset) the live block.
    pub(crate) fn absorb_hive_plan(&mut self, tasks: Vec<TaskSpec>) {
        // A zero-task plan has no telemetry to fold it — never open a block
        // for one, or is_busy() would stay latched forever. The broker's
        // plan-summary and swarm-done messages already tell the story.
        if tasks.is_empty() {
            self.swarm = None;
            return;
        }
        self.swarm = Some(SwarmStatus::new(tasks));
    }

    /// Forwarded telemetry; folds the block once the run is over.
    pub(crate) fn absorb_hive(&mut self, ev: &HiveEvent) {
        self.tools.absorb(ev, crate::chattime::unix_now_ms());
        let Some(s) = self.swarm.as_mut() else {
            return;
        };
        s.apply(ev);
        if s.finished() {
            self.fold_swarm();
        }
    }

    /// Run over (or broker gone): the live block becomes the record, the
    /// tool lines close.
    pub(crate) fn fold_swarm(&mut self) {
        self.abandon_blocks();
        if let Some(s) = self.swarm.take() {
            self.push_capped(crate::chatlayout::Message {
                sender: SENDER.into(),
                text: text(&s, crate::anim::now_ms()),
                ts: crate::chattime::unix_now_ms().to_string(),
                meta: String::new(),
                usage: None,
                expanded: false,
            });
        }
    }

    /// The aggregate Stats' tool pool: onto the live block if the run is
    /// still up, else onto the record it already folded into (its header
    /// line, once — a record that has the figure keeps it).
    pub(crate) fn note_swarm_tools(&mut self, tools: Option<(u32, u32)>) {
        let Some(words) = tools_words(tools) else {
            return;
        };
        if let Some(s) = self.swarm.as_mut() {
            s.tools = tools;
            return;
        }
        let Some(m) = self
            .messages
            .iter_mut()
            .rev()
            .find(|m| m.sender == SENDER && m.text.starts_with(LEAD))
        else {
            return;
        };
        let end = m.text.find('\n').unwrap_or(m.text.len());
        if !m.text[..end].contains(" \u{b7} tools ") {
            m.text.insert_str(end, &format!(" \u{b7} {words}"));
        }
    }
}

#[cfg(test)]
#[path = "chatswarmrec_tests.rs"]
mod tests;
