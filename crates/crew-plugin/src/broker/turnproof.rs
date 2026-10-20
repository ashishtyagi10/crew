//! What a turn's tools actually did, as a judge reads it.
//!
//! A reply's answer is the agent's claim. The judge that checks it against
//! the request (`replyverify`) needs the other half: the calls that ran and
//! how each came back. Both engines report their tools on one lane — crew's
//! own loop through `toolhive`, Claude Code's through its stream — and every
//! event passes `hoptool`, which notes it here. One log per process, like
//! `steer`'s inbox: a broker serves one session, and a verified reply clears
//! it ([`start`]) before its turn, so another task's calls reach the judge
//! only when two run at once.
use std::sync::Mutex;

use crew_hive::HiveEvent;

/// The most lines kept; an unverified session must not grow the log forever.
const KEEP: usize = 60;
/// Chars of a call's arguments, and of a result's first line, kept per line.
const HEAD: usize = 90;

static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Forget what earlier turns did: a verified reply's turn starts here.
pub(crate) fn start() {
    LOG.lock().unwrap_or_else(|p| p.into_inner()).clear();
}

/// Note `event` when it is a call or a result. A result completes the last
/// open line of its tool, `sys:run {"cmd": …} → failed: exit 101`.
pub(crate) fn note(event: &HiveEvent) {
    let mut log = LOG.lock().unwrap_or_else(|p| p.into_inner());
    match event {
        HiveEvent::ToolCall { label, args, .. } => {
            log.push(format!("{label} {}", head(args)));
            let over = log.len().saturating_sub(KEEP);
            log.drain(..over);
        }
        HiveEvent::ToolResult {
            label, ok, text, ..
        } => {
            let open = log
                .iter_mut()
                .rev()
                .find(|l| l.starts_with(label.as_str()) && !l.contains(" \u{2192} "));
            if let Some(line) = open {
                let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                let how = if *ok { "ok" } else { "failed" };
                line.push_str(&format!(" \u{2192} {how}: {}", head(first)));
            }
        }
        _ => {}
    }
}

/// The turn's calls so far, one numbered line each.
pub(crate) fn lines() -> String {
    let log = LOG.lock().unwrap_or_else(|p| p.into_inner());
    let rows: Vec<String> = log
        .iter()
        .enumerate()
        .map(|(i, l)| format!("{}. {l}", i + 1))
        .collect();
    rows.join("\n")
}

fn head(s: &str) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    match flat.chars().count() > HEAD {
        true => flat.chars().take(HEAD).collect::<String>() + "\u{2026}",
        false => flat,
    }
}

#[cfg(test)]
#[path = "turnproof_tests.rs"]
mod tests;
