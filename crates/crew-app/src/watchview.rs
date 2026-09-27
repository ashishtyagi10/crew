//! `/watching`: what crew is waiting to do on its own clock.
//!
//! The standing intents (`crew daemon at …`, or "remind me …" said over a
//! channel) lived in `watchlist.jsonl` beside the ledger and could be listed
//! and cancelled from the CLI and from a phone — and from the app, the one
//! place a person is most likely to wonder what is pending, from nowhere.
//! The goal doc's own rule: crew can list and cancel what it is watching, or
//! the feature is a haunting.
//!
//! Read straight off the log, like `/tools` reads the ledger: the daemon
//! re-reads the same file on every tick, so a cancel appended here is a
//! cancel the clock honours, daemon running or not.
use std::collections::BTreeMap;

use crate::daemon::intent::{spell, until, Intent};
use crate::daemon::intenthistory::{self, Fired};
use crate::daemon::intentlog::Watchlist;

#[cfg(test)]
#[path = "watchview_tests.rs"]
mod tests;

/// The row's fixed columns: the id, when, and the cadence; the task takes the
/// rest of a tile's width.
const ID_W: usize = 3;
const UNTIL_W: usize = 8;
const REPEAT_W: usize = 10;
/// Where the task starts, and the column its wrapped lines hang from.
const TASK_AT: usize = ID_W + 2 + UNTIL_W + 2 + REPEAT_W + 2;
/// Where a detail line's words start: past the id, and past a `→ ` lead.
const DETAIL_AT: usize = ID_W + 2 + 2;

/// The listing for `intents` (soonest first, as the log folds them), as viewer text.
/// `history` is what each has already done, by id — the fold that `live()` drops.
pub(crate) fn listing(
    intents: &[Intent],
    history: &BTreeMap<String, Fired>,
    now_ms: u64,
) -> String {
    let mut out = String::from("# watching \u{b7} what crew is waiting to do\n");
    if intents.is_empty() {
        // A headline and ONE paragraph: the viewer wraps prose on words at
        // the pane's width, and a hint broken by hand at the tile's read as
        // a ragged poem in a wide pane. Plain text, so no backticks: the
        // viewer prints them. The straight quotes are the shell's own.
        out.push_str(
            "Nothing standing.\nRun crew daemon at \"tomorrow 9am the forecast\" in a shell \
             to set one, or say \u{201c}remind me \u{2026}\u{201d} over a channel.\n",
        );
        return out;
    }
    // A snapshot, and it says so — `in 2h` is only true at the moment it was
    // written — and the way to call one off, since that is the other half of
    // why anybody opens this.
    out.push_str(&format!(
        "{} standing \u{b7} times as of opening\n/watching re-reads the list\n\
         /watching cancel <id> calls one off \u{b7} /watching snooze <id> 30m pushes one back\n\n",
        intents.len()
    ));
    // Fitted to a tile, the way `/tools` and `/blocks` fit theirs: laid out
    // as one line each, a tile's viewer wrapped the task back to column 0
    // under the id, and a detail line broke inside a part (`fired 40× ·` /
    // `last 16h ago`).
    let task_w = crate::toolsrow::ROW_W.saturating_sub(TASK_AT);
    for i in intents {
        let task = crate::toolsrow::wrap(i.text.trim(), task_w);
        for (k, line) in task.iter().enumerate() {
            match k {
                0 => out.push_str(&format!(
                    "{:<ID_W$}  {:<UNTIL_W$}  {:<REPEAT_W$}  {line}\n",
                    i.id,
                    until(i.fire_ms, now_ms),
                    i.repeat.label(),
                )),
                _ => out.push_str(&format!("{:TASK_AT$}{line}\n", "")),
            }
        }
        // Where the answer goes, when it goes somewhere other than the pane,
        // and how long this has been standing — the two things a row does
        // not already say.
        let mut parts = Vec::new();
        if !i.to.is_empty() {
            parts.push(i.to.clone());
        }
        if let Some(ms) = now_ms.checked_sub(i.created_ms) {
            parts.push(format!("standing {}", spell(ms / 1000)));
        }
        // What it has already done: a daily that has fired forty times and one set
        // this morning read the same otherwise, and a missed firing was said once on
        // its channel and then nowhere.
        if let Some(f) = history.get(&i.id) {
            parts.push(intenthistory::note(f, now_ms));
        }
        // The words line up whether or not the first part is a `→ channel`.
        let lead = if i.to.is_empty() { "  " } else { "\u{2192} " };
        for (k, line) in detail_lines(&parts).iter().enumerate() {
            let mark = if k == 0 { lead } else { "  " };
            out.push_str(&format!("{:w$}{mark}{line}\n", "", w = DETAIL_AT - 2));
        }
    }
    out
}

/// `parts` joined by ` · ` onto as few tile-wide lines as hold them, breaking
/// only BETWEEN parts.
fn detail_lines(parts: &[String]) -> Vec<String> {
    let room = crate::toolsrow::ROW_W.saturating_sub(DETAIL_AT);
    let mut out: Vec<String> = Vec::new();
    for p in parts {
        match out.last_mut() {
            Some(cur) if cur.chars().count() + 3 + p.chars().count() <= room => {
                cur.push_str(" \u{b7} ");
                cur.push_str(p);
            }
            _ => out.push(p.clone()),
        }
    }
    out
}

/// What `/watching cancel <id>` did, as the status line says it.
pub(crate) fn cancel(list: &Watchlist, id: &str, now_ms: u64) -> String {
    let id = id.trim();
    if id.is_empty() {
        return "usage: /watching cancel <id>".into();
    }
    match list.cancel(id, now_ms) {
        Ok(true) => format!("{id} cancelled"),
        Ok(false) => format!("crew is not watching for {id}"),
        Err(e) => format!("watching: cannot write the list: {e}"),
    }
}

/// What `/watching snooze <id> <for>` did, as the status line says it.
pub(crate) fn snooze(list: &Watchlist, rest: &str, now_ms: u64) -> String {
    let mut words = rest.split_whitespace();
    let (Some(id), Some(word)) = (words.next(), words.next()) else {
        return "usage: /watching snooze <id> <for>  (30m, 2h, 1d)".into();
    };
    match crate::daemon::snooze::delay_ms(word) {
        Some(delay) => crate::daemon::snooze::said(id, list.snooze(id, delay, now_ms), now_ms),
        None => crate::daemon::snooze::FOR_HOW_LONG.into(),
    }
}

impl crate::app::CrewApp {
    /// `/watching` — the standing intents in the viewer; `/watching cancel <id>` calls one
    /// off; `/watching snooze <id> <for>` pushes one back.
    pub(crate) fn open_watching(&mut self, arg: &str) {
        let list = Watchlist::at(crate::daemon::intentlog::default_path());
        let now_ms = crate::chattime::unix_now_ms();
        if let Some(id) = arg.trim().strip_prefix("cancel") {
            let said = cancel(&list, id, now_ms);
            self.set_status(said);
            return;
        }
        if let Some(rest) = arg.trim().strip_prefix("snooze") {
            let said = snooze(&list, rest, now_ms);
            self.set_status(said);
            return;
        }
        if !arg.trim().is_empty() {
            self.set_status(
                "usage: /watching \u{b7} /watching cancel <id> \u{b7} /watching snooze <id> <for>"
                    .to_string(),
            );
            return;
        }
        let text = listing(&list.live(), &list.history(), now_ms);
        let path = crate::lastout::temp_path(usize::MAX, "watching");
        if let Err(e) = std::fs::write(&path, text) {
            self.set_status(format!("watching: cannot write: {e}"));
            return;
        }
        let before = self.panes.len();
        self.open_view(&path.to_string_lossy());
        self.name_last_view("watching");
        self.mark_last_view_ephemeral(before);
    }
}
