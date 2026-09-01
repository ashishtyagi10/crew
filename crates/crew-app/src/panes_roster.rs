//! Builds the `crew panes` roster from live pane state — the directory an
//! asking agent reads to discover and choose a target. Every pane gets a
//! stable-within-session `p{i}` id plus its friendly label, kind, foreground
//! agent, dir, and busy state.
use crate::ipc_types::PaneCard;
use crate::pane::{Pane, PaneContent};
use crate::procname::ProcNames;

/// How a pane in window `w` is addressed. The first window keeps the bare `p{i}` it has always
/// had — every script and every agent that learned the spelling still works — and a second
/// window's panes are `w1p0`, `w1p1`. Positional ids are per-canvas, so without the prefix two
/// windows both answer to `p0` and an ask reaches whichever was asked first.
pub(crate) fn pane_id(window: usize, i: usize) -> String {
    match window {
        0 => format!("p{i}"),
        w => format!("w{w}p{i}"),
    }
}

/// One roster card for pane `i` of window `w`.
pub(crate) fn card_in(w: usize, i: usize, p: &Pane, procnames: &ProcNames) -> PaneCard {
    PaneCard {
        id: pane_id(w, i),
        ..card_for(i, p, procnames)
    }
}

/// One roster card for pane `i`.
pub(crate) fn card_for(i: usize, p: &Pane, procnames: &ProcNames) -> PaneCard {
    let (kind, running, busy) = match &p.content {
        PaneContent::Terminal(t) => {
            let cmd = t.pty.foreground_pid().and_then(|pid| procnames.name(pid));
            let busy = cmd.is_some(); // a foreground agent (claude/codex/…) is running
            ("terminal", cmd, busy)
        }
        PaneContent::Chat(c) => ("swarm", None, c.is_busy()),
        PaneContent::Swarm(_) => ("swarm", None, false),
        PaneContent::Far(_) => ("far", None, false),
        PaneContent::Todo(_) => ("todo", None, false),
        PaneContent::Usage(_) => ("usage", None, false),
        PaneContent::Disk(_) => ("disk", None, false),
        PaneContent::Dash(_) => ("dash", None, false),
        _ => ("other", None, false),
    };
    PaneCard {
        id: format!("p{i}"),
        label: p.name.clone().or_else(|| p.label.clone()),
        kind: kind.to_string(),
        running,
        dir: p
            .dir
            .as_ref()
            .and_then(|d| d.file_name())
            .map(|n| n.to_string_lossy().into_owned()),
        busy,
    }
}

/// The full roster.
pub(crate) fn roster(panes: &[Pane], procnames: &ProcNames) -> Vec<PaneCard> {
    roster_in(0, panes, procnames)
}

/// The full roster of window `w`.
pub(crate) fn roster_in(w: usize, panes: &[Pane], procnames: &ProcNames) -> Vec<PaneCard> {
    panes
        .iter()
        .enumerate()
        .map(|(i, p)| card_in(w, i, p, procnames))
        .collect()
}

#[cfg(test)]
#[path = "panes_roster_tests.rs"]
mod tests;
