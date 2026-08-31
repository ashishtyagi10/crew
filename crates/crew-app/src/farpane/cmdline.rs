//! The `/far` pane's command line: tab completion over binaries on `PATH`,
//! history, the ghost suggestion, and shell-escaping what you send.
//!
//! Split from [`super::keys`] for the line cap, along the line between keys
//! that drive the PANEL and keys that drive the line you type into.
use super::keys::FarAction;
use super::FarPane;

/// Esc on the command line: cancel an active Tab-cycle (restoring the
/// pre-cycle text) if one is running; else clear a typed command; else ask
/// the app to close the pane.
pub(crate) fn escape_cmdline(p: &mut FarPane) -> Option<FarAction> {
    if let Some(state) = p.complete.take() {
        p.cmdline = state.prefix;
        return None;
    }
    // A landed suggestion restores the original `!` text verbatim and
    // discards the suggestion. A still-thinking ask just cancels (its
    // worker thread finishes in the background but the result is dropped)
    // and falls through to the normal clear/close behaviour below.
    if let Some(super::ask::AskState::Suggested { original }) = p.ask.take() {
        p.cmdline = original;
        return None;
    }
    if !p.cmdline.is_empty() {
        p.cmdline.clear();
        return None;
    }
    Some(FarAction::Close)
}

/// Tab while the command line has text: cycle an existing candidate list, or
/// build a fresh one from the caret token. A single candidate applies
/// immediately (trailing space unless it's a directory, so deeper completion
/// chains); more than one starts a cycle on the first candidate, and another
/// Tab advances it (wrapping).
pub(crate) fn tab_complete(p: &mut FarPane) {
    // Any bar edit cancels a thinking ask / demotes a landed suggestion —
    // otherwise a reply landing later would clobber the tabbed text (and an
    // Enter aimed at it could run a suggestion the user never read).
    p.ask = None;
    if let Some(state) = &mut p.complete {
        state.i = (state.i + 1) % state.candidates.len();
        let candidate = state.candidates[state.i].clone();
        p.cmdline = super::complete::apply(&state.prefix, &candidate);
        return;
    }
    let (kind, _token) = super::complete::caret_token(&p.cmdline);
    let binaries = command_binaries(p, kind);
    let candidates = super::complete::candidates(&p.cmdline, &p.active_cwd(), &binaries);
    if candidates.is_empty() {
        return;
    }
    if candidates.len() == 1 {
        p.cmdline = super::complete::apply(&p.cmdline, &candidates[0]);
        if !p.cmdline.ends_with('/') {
            p.cmdline.push(' ');
        }
        return;
    }
    let prefix = p.cmdline.clone();
    p.cmdline = super::complete::apply(&prefix, &candidates[0]);
    p.complete = Some(super::complete::CycleState {
        candidates,
        i: 0,
        prefix,
    });
}

/// Command-kind completion needs the cached `$PATH` binaries; kick off the
/// background scan on first use (returns builtins-only until it lands, never
/// blocking this thread). Path-kind completion needs no binaries at all.
/// `p.bins` is the session-wide cache (see `FarPane::bins`/`shared_bins`), so
/// if another pane's scan already landed this returns instantly without
/// spawning anything here.
fn command_binaries(p: &mut FarPane, kind: super::complete::TokenKind) -> Vec<String> {
    if kind != super::complete::TokenKind::Command {
        return Vec::new();
    }
    if let Some(bins) = p.bins.get() {
        return bins.clone();
    }
    if !p.bins_scan_started {
        p.bins_scan_started = true;
        let slot = p.bins.clone();
        std::thread::spawn(move || {
            let path_var = std::env::var("PATH").unwrap_or_default();
            let bins = super::complete::scan_path_binaries(&path_var);
            let _ = slot.set(bins);
        });
    }
    Vec::new()
}

/// Up while typing: recall the previous (older) history entry into the
/// command line, stashing the currently-typed text so Down can restore it.
pub(crate) fn history_prev(p: &mut FarPane) {
    if let Some(s) = p.history.prev(&p.cmdline) {
        p.cmdline = s.to_string();
    }
    p.complete = None;
    p.ask = None; // recall is an edit: cancel/demote any ask (see tab_complete)
}

/// Down while typing: recall the next (newer) history entry, or restore the
/// text that was being typed once past the newest entry.
pub(crate) fn history_next(p: &mut FarPane) {
    if let Some(s) = p.history.next(&p.cmdline) {
        p.cmdline = s.to_string();
    }
    p.complete = None;
    p.ask = None; // recall is an edit: cancel/demote any ask (see tab_complete)
}

/// Right/End while typing: accept the visible ghost-text history suggestion
/// into the command line, if one is showing. `render.rs` suppresses the
/// ghost display while a Tab-cycle is active (the candidate list already
/// occupies the line), so during a cycle this must only end the cycle — a
/// ghost lookup here would insert a suggestion that was never on screen.
pub(crate) fn accept_ghost(p: &mut FarPane) {
    p.ask = None; // ghost accept is an edit: cancel/demote any ask (see tab_complete)
    if p.complete.take().is_some() {
        return;
    }
    if let Some(g) = p.history.ghost(&p.cmdline) {
        p.cmdline = g.to_string();
    }
}
