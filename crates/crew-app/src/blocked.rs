//! "Waiting on a human" detection: which panes are blocked on the user (an
//! approval prompt, a y/n question, a pending plan) and the policy for
//! surfacing them (attention badge, one auto-focus per episode, Cmd+. cycle).
//!
//! The hard part is waiting-vs-thinking: an agent quietly computing must NOT
//! read as blocked. The terminal heuristic is deliberately conservative
//! (prefer false negatives): a pane counts only when its PTY has been quiet
//! for [`QUIET_MS`] AND the visible grid tail carries a question/permission
//! signal — a bare `❯`/`$` prompt is just an idle shell. The smith/chat pane
//! is exact: its broker reports a pending Plan (`plan_pending`).
use crate::pane::{Pane, PaneContent};
use crew_term::TermModel;

/// PTY output must be quiet this long before a terminal can read as waiting.
pub(crate) const QUIET_MS: u64 = 3_000;
/// The user must be hands-off this long before auto-focus may move.
pub(crate) const USER_IDLE_MS: u64 = 5_000;
/// Detection cadence: the (comparatively) costly row scan runs at most 1×/s.
pub(crate) const CHECK_EVERY_MS: u64 = 1_000;
/// How many trailing non-empty grid rows the prompt matcher inspects.
pub(crate) const TAIL_ROWS: usize = 5;

/// Lowercase substrings that mark a question the pane is holding for the
/// user. One const so the list is extensible and testable; matched against
/// lowercased rows, so `(y/n)` covers `[y/N]`, `(Y/n)`, … via the bracket
/// variants below.
pub(crate) const PROMPT_PATTERNS: &[&str] = &[
    "(y/n",
    "[y/n",
    "(yes/no",
    "[yes/no",
    "do you want",
    "would you like",
    "proceed?",
    "continue?",
    "press enter",
    "press return",
];
/// Permission language that, near a `?`, marks an approval prompt.
const PERMISSION_WORDS: &[&str] = &["allow", "approve", "permission"];

/// The last [`TAIL_ROWS`] non-empty rows, trimmed and lowercased.
fn tail(rows: &[String]) -> Vec<String> {
    let non_empty: Vec<String> = rows
        .iter()
        .map(|r| r.trim().to_lowercase())
        .filter(|r| !r.is_empty())
        .collect();
    let skip = non_empty.len().saturating_sub(TAIL_ROWS);
    non_empty.into_iter().skip(skip).collect()
}

/// Whether the visible tail of a grid (rows as text, top to bottom) looks
/// like a prompt waiting on the user. Only the last [`TAIL_ROWS`] non-empty
/// rows count, so an old `(y/n)` scrolled above fresh output never matches.
pub(crate) fn tail_is_prompt(rows: &[String]) -> bool {
    let tail = tail(rows);
    let Some(last) = tail.last() else {
        return false;
    };
    if tail
        .iter()
        .any(|r| PROMPT_PATTERNS.iter().any(|p| r.contains(p)))
    {
        return true;
    }
    // A `❯` selector on the LAST row under an earlier question (agent
    // approval menus). A bare `❯` shell prompt has no question above it.
    if last.contains('❯') && tail[..tail.len() - 1].iter().any(|r| r.contains('?')) {
        return true;
    }
    // Permission language anywhere in the tail alongside a question mark.
    tail.iter().any(|r| r.contains('?'))
        && tail
            .iter()
            .any(|r| PERMISSION_WORDS.iter().any(|w| r.contains(w)))
}

/// The terminal-pane waiting predicate: quiescent for [`QUIET_MS`] AND the
/// grid tail matches a prompt.
pub(crate) fn term_waiting(now: u64, last_output_ms: u64, rows: &[String]) -> bool {
    now.saturating_sub(last_output_ms) >= QUIET_MS && tail_is_prompt(rows)
}

/// Whether `p` is blocked on the user right now. Chat panes are exact
/// (`plan_pending`); terminals gate the row scan on a running foreground
/// command (an idle shell is never waiting, even with an old question above
/// its prompt) and an unscrolled view (scrolled back, the viewport tail is
/// not the live tail), then on quiescence — so the O(cells) scan only runs
/// on panes that have already gone quiet.
pub(crate) fn pane_blocked(p: &Pane, now: u64) -> bool {
    match &p.content {
        PaneContent::Chat(c) => c.plan_pending,
        PaneContent::Terminal(t) => {
            if t.cmd.is_none()
                || t.pty.display_offset() != 0
                || now.saturating_sub(t.last_output_ms) < QUIET_MS
            {
                return false;
            }
            let rows =
                crate::search::rows_text(&t.pty.cells(false), p.grid.cols, p.grid.rows, false);
            term_waiting(now, t.last_output_ms, &rows)
        }
        _ => false,
    }
}

/// What one [`BlockedState::update`] decided: panes that just became blocked
/// (badge these) and at most one pane to auto-focus.
#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Update {
    pub(crate) newly: Vec<u64>,
    pub(crate) focus: Option<u64>,
}

/// One pane's blocked episode: whether it read as blocked last check, and
/// whether this episode has already been surfaced (seen focused, or
/// auto-focused once) — the flag that stops focus ping-pong.
struct Episode {
    key: u64,
    blocked: bool,
    surfaced: bool,
}

/// Per-app episode tracking, keyed by pane `born_ms` (the pane-identity
/// scheme the rest of the codebase uses — indices shift on close).
#[derive(Default)]
pub(crate) struct BlockedState {
    next_check_ms: u64,
    episodes: Vec<Episode>,
}

impl BlockedState {
    /// Throttle: at most one check per [`CHECK_EVERY_MS`].
    pub(crate) fn due(&mut self, now: u64) -> bool {
        if now < self.next_check_ms {
            return false;
        }
        self.next_check_ms = now + CHECK_EVERY_MS;
        true
    }

    /// Fold this tick's per-pane blocked snapshot into the episode state.
    /// A pane is `newly` blocked on its false→true edge while unfocused (the
    /// focused pane is already being looked at). Auto-focus fires for at most
    /// one newly blocked pane, only while the user is idle and the focused
    /// pane is not itself blocked — and once per episode.
    pub(crate) fn update(
        &mut self,
        snap: &[(u64, bool)],
        focused_key: Option<u64>,
        focused_blocked: bool,
        user_idle: bool,
    ) -> Update {
        self.episodes
            .retain(|e| snap.iter().any(|&(k, _)| k == e.key));
        let mut up = Update::default();
        for &(key, blocked) in snap {
            let ep = match self.episodes.iter_mut().find(|e| e.key == key) {
                Some(e) => e,
                None => {
                    self.episodes.push(Episode {
                        key,
                        blocked: false,
                        surfaced: false,
                    });
                    self.episodes.last_mut().expect("just pushed")
                }
            };
            let newly = blocked && !ep.blocked;
            ep.blocked = blocked;
            if !blocked {
                ep.surfaced = false; // the episode ended; the next one is fresh
                continue;
            }
            if focused_key == Some(key) {
                ep.surfaced = true; // the user is already looking at it
                continue;
            }
            if newly {
                up.newly.push(key);
                if up.focus.is_none() && user_idle && !focused_blocked && !ep.surfaced {
                    ep.surfaced = true;
                    up.focus = Some(key);
                }
            }
        }
        up
    }
}

#[cfg(test)]
#[path = "blocked_tests.rs"]
mod tests;
