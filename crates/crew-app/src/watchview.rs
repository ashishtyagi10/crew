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
use crate::daemon::intent::{spell, until, Intent};
use crate::daemon::intentlog::Watchlist;

#[cfg(test)]
#[path = "watchview_tests.rs"]
mod tests;

/// The row's fixed columns: the id, when, and the cadence; the task takes the
/// rest of a tile's width.
const ID_W: usize = 3;
const UNTIL_W: usize = 8;
const REPEAT_W: usize = 10;

/// The listing for `intents` (soonest first, as the log folds them), as viewer text.
pub(crate) fn listing(intents: &[Intent], now_ms: u64) -> String {
    let mut out = String::from("# watching \u{b7} what crew is waiting to do\n");
    if intents.is_empty() {
        out.push_str(
            "Nothing standing.\n`crew daemon at \"tomorrow 9am the forecast\"` sets one,\n\
             and so does \u{201c}remind me \u{2026}\u{201d} said over a channel.\n",
        );
        return out;
    }
    // A snapshot, and it says so — `in 2h` is only true at the moment it was
    // written — and the way to call one off, since that is the other half of
    // why anybody opens this.
    out.push_str(&format!(
        "{} standing \u{b7} times as of opening\n/watching re-reads the list\n\
         /watching cancel <id> calls one off\n\n",
        intents.len()
    ));
    for i in intents {
        out.push_str(&format!(
            "{:<ID_W$}  {:<UNTIL_W$}  {:<REPEAT_W$}  {}\n",
            i.id,
            until(i.fire_ms, now_ms),
            i.repeat.label(),
            i.text.trim(),
        ));
        // Where the answer goes, when it goes somewhere other than the pane,
        // and how long this has been standing — the two things a row does
        // not already say.
        let mut parts = Vec::new();
        if !i.to.is_empty() {
            parts.push(format!("\u{2192} {}", i.to));
        }
        if let Some(ms) = now_ms.checked_sub(i.created_ms) {
            parts.push(format!("standing {}", spell(ms / 1000)));
        }
        if !parts.is_empty() {
            out.push_str(&format!(
                "{:indent$}{}\n",
                "",
                parts.join(" \u{b7} "),
                indent = ID_W + 2
            ));
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

impl crate::app::CrewApp {
    /// `/watching` — the standing intents in the viewer; `/watching cancel <id>` calls one off.
    pub(crate) fn open_watching(&mut self, arg: &str) {
        let list = Watchlist::at(crate::daemon::intentlog::default_path());
        let now_ms = crate::chattime::unix_now_ms();
        if let Some(id) = arg.trim().strip_prefix("cancel") {
            let said = cancel(&list, id, now_ms);
            self.set_status(said);
            return;
        }
        if !arg.trim().is_empty() {
            self.set_status("usage: /watching \u{b7} /watching cancel <id>".to_string());
            return;
        }
        let text = listing(&list.live(), now_ms);
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
