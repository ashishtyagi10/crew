//! What a terminal pane is doing, in a few words: the second line of its
//! entry in the nav's PANES list (`navpanecard`).
//!
//! The pane's last line was the obvious answer and the wrong one for the
//! panes that matter most: an agent CLI's last line is its footer
//! (`? for shortcuts`, the mode line), the same on every pane. What it is
//! DOING sits a few rows up, and has a shape each agent shares:
//! - working: a spinner line with a timer and an interrupt hint
//!   (`✳ Pondering… (12s · esc to interrupt)`, Codex's `• Working (12s • esc
//!   to interrupt)`, Gemini's `⠏ Thinking... (esc to cancel, 12s)`) —
//!   said as `Pondering… · 12s`;
//! - waiting on you: the question its dialog asks (the same tail
//!   [`crate::blocked::tail_is_prompt`] reads);
//! - otherwise: its last message (`⏺ Updated 3 files`), or for any other
//!   program the last row that is not chrome.
//!
//! Read from the rows [`crate::blocked::observe`] already extracts once a
//! second, so it costs no extra screen walk; and a line that changed fifteen
//! times a second could not be read anyway.
use std::hash::{Hash, Hasher};

/// How long a terminal counts as working after its tail last changed. The
/// watch steps once a second, so this spans two steps: a timer ticking every
/// second (an agent thinking, a build printing) never drops out between them.
pub(crate) const MOVING_MS: u64 = 2_500;

/// Rows from the bottom the summary looks through: an agent's spinner line
/// sits under its input box (≤6 rows up), its last message a few above that.
const LOOK_ROWS: usize = 24;

/// Markers an agent CLI leads its messages with: Claude Code's `⏺`, Codex's
/// `•`, Gemini's `✦`, and the `●` some builds draw for `⏺`.
const MESSAGE: [char; 4] = ['⏺', '•', '✦', '●'];

/// Footers and hints that are on screen whatever the agent is doing.
const CHROME: [&str; 6] = [
    "for shortcuts",
    "accept edits",
    "bypass permissions",
    "plan mode",
    "to interrupt",
    "esc to cancel",
];

/// A terminal's gist: the summary line and when its tail last moved.
#[derive(Default, Debug, Clone)]
pub struct Gist {
    pub(crate) summary: Option<String>,
    hash: u64,
    /// When the tail last changed; `None` until it is first seen.
    moved_ms: Option<u64>,
    /// [`Gist::moving`] at the last observation, so a lapse is news.
    was_moving: bool,
}

impl Gist {
    /// Fold one observation of the screen's rows (top to bottom) in. `true`
    /// when what the entry shows changed — its words, or working lapsing to
    /// still — so the nav repaints though no output arrived to make it.
    pub(crate) fn note(&mut self, rows: &[String], now: u64) -> bool {
        let mut h = std::hash::DefaultHasher::new();
        rows.iter()
            .rev()
            .take(LOOK_ROWS)
            .for_each(|r| r.hash(&mut h));
        let h = h.finish();
        if h != self.hash {
            self.hash = h;
            self.moved_ms = Some(now);
        }
        let said = summary(rows);
        let moving = self.moving(now);
        let news = said != self.summary || moving != self.was_moving;
        (self.summary, self.was_moving) = (said, moving);
        news
    }

    /// Nothing is running: no summary, and nothing moving. `true` when there
    /// was something to forget.
    pub(crate) fn clear(&mut self) -> bool {
        let had = self.moved_ms.is_some();
        *self = Self::default();
        had
    }

    /// The tail moved within [`MOVING_MS`]: output arriving, a timer ticking.
    pub(crate) fn moving(&self, now: u64) -> bool {
        self.moved_ms
            .is_some_and(|m| now.saturating_sub(m) < MOVING_MS)
    }

    /// A gist for `rows` whose tail has long been still (shots and tests,
    /// whose clock starts at zero and cannot reach back).
    #[cfg(test)]
    pub(crate) fn settled(rows: &[String]) -> Self {
        Self {
            summary: summary(rows),
            hash: 1,
            ..Self::default()
        }
    }
}

/// The few words for a screen (rows top to bottom), or `None` when it shows
/// nothing worth saying.
pub(crate) fn summary(rows: &[String]) -> Option<String> {
    let tail: Vec<&str> = rows
        .iter()
        .rev()
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
        .take(LOOK_ROWS)
        .collect();
    if let Some(s) = tail.iter().find_map(|r| spinner(r)) {
        return Some(s);
    }
    if crate::blocked::tail_is_prompt(rows) {
        if let Some(q) = tail.iter().find(|r| r.contains('?') && !chrome(r)) {
            return crate::minpreview::tidy(unboxed(q));
        }
    }
    let message = tail.iter().find_map(|r| {
        let rest = r.strip_prefix(MESSAGE)?;
        crate::minpreview::tidy(rest)
    });
    message.or_else(|| {
        tail.iter()
            .map(|r| unboxed(r))
            .find(|r| !chrome(r) && r.chars().any(char::is_alphanumeric))
            .and_then(crate::minpreview::tidy)
    })
}

/// An agent's working line, as `Pondering… · 12s`: the words before its
/// parenthesis, the spinner glyph dropped, and the first time in it.
fn spinner(row: &str) -> Option<String> {
    let lower = row.to_lowercase();
    if !(lower.contains("to interrupt") || lower.contains("esc to cancel")) {
        return None;
    }
    let (head, rest) = row.split_once('(')?;
    // The spinner glyph before it and the agent's own `…` after it go: the
    // time that follows says it is still going, and a clip adds its own.
    let head = head
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim();
    let head = head.trim_end_matches(['…', '.']).trim_end();
    if head.is_empty() {
        return None;
    }
    let time = rest
        .split(|c: char| c == '·' || c == '•' || c == ',' || c == ')' || c.is_whitespace())
        .find(|w| is_elapsed(w));
    Some(match time {
        Some(t) => format!("{head} \u{b7} {t}"),
        None => head.to_string(),
    })
}

/// A summary's words and its elapsed time, when it ends in one
/// (`Pondering · 12s` → `("Pondering", Some("12s"))`).
pub(crate) fn split_time(s: &str) -> (&str, Option<&str>) {
    match s.rsplit_once(" \u{b7} ") {
        Some((head, t)) if is_elapsed(t) => (head, Some(t)),
        _ => (s, None),
    }
}

/// `12s`, `1m`, `3m20s`, `2h`: digits and unit letters, a digit first.
fn is_elapsed(w: &str) -> bool {
    w.starts_with(|c: char| c.is_ascii_digit())
        && w.ends_with(['s', 'm', 'h'])
        && w.chars().all(|c| c.is_ascii_digit() || "smh".contains(c))
}

/// A row with a box's sides stripped: `│ Do you want to proceed? │`.
fn unboxed(row: &str) -> &str {
    row.trim_matches(|c: char| c == '│' || c == '┃' || c.is_whitespace())
}

/// A footer, a hint, a box edge or an empty prompt: on screen whatever the
/// program is doing, so never what it is doing.
fn chrome(row: &str) -> bool {
    let lower = row.to_lowercase();
    let inner = unboxed(row);
    CHROME.iter().any(|c| lower.contains(c)) || inner.chars().all(|c| "─━═╭╮╰╯┌┐└┘>❯› ".contains(c))
}

#[cfg(test)]
#[path = "panegist_tests.rs"]
mod tests;
