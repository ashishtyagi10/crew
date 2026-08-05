//! "Waiting on a human" detection: which panes are blocked on the user (an
//! approval prompt, a y/n question, a pending plan) and the policy for
//! surfacing them (attention badge, one auto-focus per episode, Cmd+. cycle).
//!
//! The hard part is waiting-vs-thinking: an agent quietly computing must NOT
//! read as blocked. PTY-byte quiescence cannot draw that line: Claude Code
//! repaints a blinking `⏺` every ~600 ms while its approval dialog just sits
//! there (measured 2026-08-05, claude 2.1.222 — 47–49 bytes each ~0.6 s), so
//! a byte-quiet gate never opens. What actually distinguishes the two states
//! is the RENDERED bottom of the screen: waiting, its text is frozen;
//! thinking, a spinner/timer line (`✳ Pondering… (12s · esc to interrupt)`)
//! rewrites it every second. So quiescence is text stability: the last
//! [`STABLE_ROWS`] non-empty rows unchanged for [`QUIET_MS`] (blinks and OSC
//! title churn don't alter that text; a ticking timer does). A pane counts as
//! blocked only when that stable tail also carries a question/permission
//! signal within [`MATCH_ROWS`] rows — wide enough to reach a question pushed
//! up by wrapped option lines in a narrow pane (Claude Code's dialog puts it
//! 8 non-empty rows above the bottom at 44 cols) or by Codex's
//! press-enter/composer footer. The smith/chat pane is exact: its broker
//! reports a pending Plan (`plan_pending`).
use crate::pane::{Pane, PaneContent};
use crew_term::TermModel;
use std::hash::{Hash, Hasher};

/// The rendered tail must be text-stable this long before a terminal can
/// read as waiting.
pub(crate) const QUIET_MS: u64 = 3_000;
/// The user must be hands-off this long before auto-focus may move.
pub(crate) const USER_IDLE_MS: u64 = 5_000;
/// Detection cadence: the (comparatively) costly row scan runs at most 1×/s.
pub(crate) const CHECK_EVERY_MS: u64 = 1_000;
/// How many trailing non-empty grid rows the prompt matcher inspects. Sized
/// for real agent dialogs: Claude Code's question sits ≤8 non-empty rows up
/// in a 44-col pane (wrapped options + hint), Codex's ≤10 (options + footer).
pub(crate) const MATCH_ROWS: usize = 12;
/// How many trailing non-empty rows the stability hash covers. Must reach the
/// thinking-state spinner/timer line (≤6 rows above the bottom, under the
/// input box) but NOT the blinking `⏺` tool-call line above a waiting
/// approval dialog (≥10 non-empty rows up: separator, command, description,
/// question, options, hint).
pub(crate) const STABLE_ROWS: usize = 8;

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

/// The last `n` non-empty rows, trimmed and lowercased.
fn tail(rows: &[String], n: usize) -> Vec<String> {
    let non_empty: Vec<String> = rows
        .iter()
        .map(|r| r.trim().to_lowercase())
        .filter(|r| !r.is_empty())
        .collect();
    let skip = non_empty.len().saturating_sub(n);
    non_empty.into_iter().skip(skip).collect()
}

/// Whether the visible tail of a grid (rows as text, top to bottom) looks
/// like a prompt waiting on the user. Only the last [`MATCH_ROWS`] non-empty
/// rows count, so an old `(y/n)` scrolled well above fresh output never
/// matches. (One deeper on-screen is tolerated: the stability gate keeps a
/// still-running command from surfacing until its output stops moving.)
pub(crate) fn tail_is_prompt(rows: &[String]) -> bool {
    let tail = tail(rows, MATCH_ROWS);
    if tail.is_empty() {
        return false;
    }
    if tail
        .iter()
        .any(|r| PROMPT_PATTERNS.iter().any(|p| r.contains(p)))
    {
        return true;
    }
    // A `❯`/`›` selector row under an earlier question: the option menu of an
    // agent approval dialog (Claude Code `❯ 1. Yes`, Codex `› 1. Yes, proceed`),
    // matched at any tail depth because option lines wrap in narrow panes. A
    // bare `❯` shell prompt has no question above it.
    if let Some(i) = tail
        .iter()
        .rposition(|r| r.starts_with('❯') || r.starts_with('›'))
    {
        if tail[..i].iter().any(|r| r.contains('?')) {
            return true;
        }
    }
    // Permission language anywhere in the tail alongside a question mark.
    tail.iter().any(|r| r.contains('?'))
        && tail
            .iter()
            .any(|r| PERMISSION_WORDS.iter().any(|w| r.contains(w)))
}

/// Per-terminal waiting detector: the rendered-tail stability clock plus the
/// last prompt-match verdict. Owned by each `TermPane`, stepped at most 1×/s
/// by [`observe`], read by [`pane_blocked`].
#[derive(Default)]
pub struct TailWatch {
    /// Hash of the last [`STABLE_ROWS`] non-empty rows at the previous step.
    hash: u64,
    /// When that hash last changed — the start of the current stable run.
    since_ms: u64,
    /// Whether the tail matched [`tail_is_prompt`] at the previous step.
    prompt: bool,
}

impl TailWatch {
    /// Fold one rendered-grid observation in: restart the stability clock if
    /// the tail text moved, and re-run the prompt matcher.
    pub(crate) fn step(&mut self, rows: &[String], now: u64) {
        let mut h = std::hash::DefaultHasher::new();
        tail(rows, STABLE_ROWS).hash(&mut h);
        let h = h.finish();
        if h != self.hash {
            self.hash = h;
            self.since_ms = now;
        }
        self.prompt = tail_is_prompt(rows);
    }

    /// The pane cannot currently be waiting (idle shell, scrolled back):
    /// clear the verdict and restart the clock.
    pub(crate) fn reset(&mut self, now: u64) {
        self.hash = 0;
        self.since_ms = now;
        self.prompt = false;
    }

    /// Waiting = the tail carries a prompt AND has been text-stable for
    /// [`QUIET_MS`].
    pub(crate) fn waiting(&self, now: u64) -> bool {
        self.prompt && now.saturating_sub(self.since_ms) >= QUIET_MS
    }
}

/// Advance a pane's [`TailWatch`] (terminals only; call at most 1×/s — the
/// row extraction is the O(cells) cost this module throttles). An idle shell
/// (`cmd` = None) is never waiting even with an old question above its
/// prompt, and a scrolled-back viewport's tail is not the live tail — both
/// reset rather than step.
pub(crate) fn observe(p: &mut Pane, now: u64) {
    let PaneContent::Terminal(t) = &mut p.content else {
        return;
    };
    if t.cmd.is_none() || t.pty.display_offset() != 0 {
        t.tail.reset(now);
        return;
    }
    let rows = crate::search::rows_text(&t.pty.cells(false), p.grid.cols, p.grid.rows, false);
    t.tail.step(&rows, now);
}

/// Whether `p` is blocked on the user right now. Chat panes are exact
/// (`plan_pending`); terminals read the [`TailWatch`] their 1 Hz [`observe`]
/// keeps fresh.
pub(crate) fn pane_blocked(p: &Pane, now: u64) -> bool {
    match &p.content {
        PaneContent::Chat(c) => c.plan_pending,
        PaneContent::Terminal(t) => t.tail.waiting(now),
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
    /// focused pane is already being looked at). Auto-focus is LEVEL-triggered,
    /// not edge-triggered: at most one unsurfaced blocked pane gets focus on
    /// any tick where the user is idle and the focused pane is not itself
    /// blocked — once per episode. (Edge-triggered focus silently forfeited
    /// the move whenever a pane blocked within [`USER_IDLE_MS`] of the user's
    /// last keystroke — i.e. almost always, in an actively used app.)
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
            }
            if up.focus.is_none() && user_idle && !focused_blocked && !ep.surfaced {
                ep.surfaced = true;
                up.focus = Some(key);
            }
        }
        up
    }
}

#[cfg(test)]
#[path = "blocked_tests.rs"]
mod tests;
