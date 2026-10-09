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

/// One task's row as plain text — the record's line, always whole. What it
/// waits on is held to the title's last word by no-break spaces: wrapped
/// in a narrow card, `← 1` alone on a row read as a row of its own.
pub(crate) fn plain(s: &SwarmStatus, i: usize) -> String {
    let (glyph, ..) = crate::swarm::view::state_style(s.tasks[i].state);
    let (spec, title, deps) = crate::chatswarmrows::words(s, i, usize::MAX, 0);
    let deps = deps.replace(' ', "\u{a0}");
    let w = s.tasks.len().to_string().len();
    format!(" {:>w$} {glyph} {spec}{title}{deps}", i + 1)
}

/// The card's text at `now_ms`: `swarm · 3 tasks · 2 done · 1 failed · 12s
/// · tools 5/12`, then one line per task ([`plain`]) — a failed one followed
/// by why (`chatswarmwhy::record_line`), as the live block drew it.
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
    let rows: Vec<String> = (0..s.tasks.len())
        .flat_map(|i| std::iter::once(plain(s, i)).chain(crate::chatswarmwhy::record_line(s, i)))
        .collect();
    format!("{LEAD}{}\n{}", head.join(" \u{b7} "), rows.join("\n"))
}

/// Columns of the specialist at the head of `rest` (a task row past its
/// glyph): its name and the padding after it, which is always two spaces or
/// more (`chatswarmrows::words`), so a title's wrap hangs under the title
/// rather than under `critic`. 0 when the row has no specialist column.
fn spec_w(rest: &[char]) -> usize {
    let Some(p) = rest.windows(2).position(|w| w == [' ', ' ']) else {
        return 0;
    };
    p + rest[p..].iter().take_while(|c| **c == ' ').count()
}

/// The record laid out for a card `width` wide: the head wrapped as prose,
/// each task row hanging its continuation under its title, each reason under
/// itself. Through markdown a wrapped row restarted at column 0, under the
/// numbers (`light ← 1` beneath `3 ×`), where it read as a row of its own.
pub(crate) fn card_lines(
    text: &str,
    width: usize,
    fg: (u8, u8, u8),
) -> Vec<crate::chatbody::CardLine> {
    let plain = |c: char| crate::chatbody::plain(c, fg, false);
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        // A title's `code` keeps no ticks off the markdown path, as the
        // footer's do not (`summaryroute::unticked`).
        let line = crate::summaryroute::unticked(line);
        let line = line.strip_prefix(' ').unwrap_or(&line);
        let chars: Vec<char> = line.chars().collect();
        // A row hangs under its title (` 12 ✓ ` is the number column, its
        // glyph and a space); a reason under its own no-break indent; the
        // head under nothing.
        let glyph = chars.iter().position(|c| !c.is_ascii_digit() && *c != ' ');
        let lead = match glyph {
            // The head's tail hangs two in: at the margin `1 cancelled ·
            // 1s` sat over `1 ✓ scout` and read as task 1.
            _ if i == 0 => 2,
            Some(g) if chars[..g].iter().any(char::is_ascii_digit) => {
                g + 2 + spec_w(&chars[g + 2..])
            }
            _ => chars.iter().take_while(|c| **c == '\u{a0}').count(),
        };
        let lead = lead.min(width / 2);
        for (k, (a, b)) in crate::viewpane::plainrung::hanging(&chars, width, lead)
            .into_iter()
            .enumerate()
        {
            let pad = if k == 0 { 1 } else { 1 + lead };
            let mut row: Vec<_> = (0..pad).map(|_| plain(' ')).collect();
            row.extend(chars[a..b].iter().map(|&c| plain(c)));
            out.push(row);
        }
    }
    out
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

#[cfg(test)]
#[path = "chatswarmrechang_tests.rs"]
mod hang_tests;
