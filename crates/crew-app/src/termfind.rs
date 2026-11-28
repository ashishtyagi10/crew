//! The find bar's state for a terminal pane: the query, every match of it in
//! the whole buffer (scrollback included, newest first), and which one is
//! current. Cmd+F opens it on the focused terminal ([`crate::termfindkeys`]);
//! [`crate::termfindview`] draws it.
//!
//! Matches come from the terminal's own grid search ([`PtyTerm::find_all`],
//! alacritty's regex walk, smart-case) rather than from the cells on screen,
//! so a match 9,000 lines back counts and can be stepped to. They are kept as
//! grid coordinates, and the current one is also pinned by its ABSOLUTE line
//! (grid line + the screen's top line), because new output shifts every grid
//! line: without the pin, a build printing while you read `3/17` would slide
//! the selection onto another match.
use crew_term::{FindSpan, PtyTerm};

/// A key the bar understands; the winit event is read once, in
/// [`crate::termfindkeys`], so everything below is testable without one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FindKey {
    Char(char),
    Backspace,
    /// Enter, Up: the next match back in time.
    Older,
    /// Shift+Enter, Down: the next match toward the live bottom.
    Newer,
    Close,
    /// Any other key: eaten, never forwarded to the program.
    Other,
}

#[derive(Default)]
pub(crate) struct TermFind {
    pub query: String,
    /// Newest first, as the terminal returns them.
    pub matches: Vec<FindSpan>,
    /// Index of the current match in `matches` (0 = the newest).
    pub sel: usize,
    /// The current match's absolute line and column, to find it again.
    pin: Option<(i64, u16)>,
    /// Output has landed since the last search, and when that search ran:
    /// a full-buffer walk is milliseconds, so a flood re-runs it at most
    /// every [`REFRESH_MS`], not on every tick.
    dirty: bool,
    last: Option<std::time::Instant>,
}

/// The least time between two searches triggered by new output.
const REFRESH_MS: u128 = 150;

/// One run of one viewport row to tint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tint {
    pub row: u16,
    pub from: u16,
    pub to: u16,
    pub current: bool,
}

impl TermFind {
    /// Take a fresh match list. `top` is the screen's top line in absolute
    /// numbering ([`PtyTerm::screen_line`]): the current match stays on the
    /// same text when it is still there, else the index is clamped.
    pub fn set_matches(&mut self, matches: Vec<FindSpan>, top: u64) {
        let abs = |m: &FindSpan| (i64::from(m.line) + top as i64, m.col);
        if let Some(pin) = self.pin {
            if let Some(i) = matches.iter().position(|m| abs(m) == pin) {
                self.sel = i;
            }
        }
        self.sel = self.sel.min(matches.len().saturating_sub(1));
        self.pin = matches.get(self.sel).map(abs);
        self.matches = matches;
    }

    /// Re-run the search against `pty`'s buffer as it is now.
    pub fn refresh(&mut self, pty: &PtyTerm) {
        (self.dirty, self.last) = (false, Some(std::time::Instant::now()));
        self.set_matches(pty.find_all(&self.query), pty.screen_line());
    }

    /// A typed character, or `None` for backspace; the selection restarts at
    /// the newest match (call [`TermFind::refresh`] after).
    pub fn edit(&mut self, c: Option<char>) {
        match c {
            Some(c) => self.query.push(c),
            None => drop(self.query.pop()),
        }
        self.sel = 0;
        self.pin = None;
    }

    /// Step one match back in time (`older`) or toward the bottom, wrapping.
    pub fn step(&mut self, older: bool, top: u64) {
        let n = self.matches.len();
        if n > 0 {
            self.sel = (self.sel + if older { 1 } else { n - 1 }) % n;
            self.pin = self
                .matches
                .get(self.sel)
                .map(|m| (i64::from(m.line) + top as i64, m.col));
        }
    }

    /// `3/17` (k counted from the newest), or `no matches` — the second
    /// flagged so the bar can wear the bell colour. `None` before anything is
    /// typed.
    pub fn count(&self) -> Option<(String, bool)> {
        match (self.query.is_empty(), self.matches.len()) {
            (true, _) => None,
            (false, 0) => Some(("no matches".into(), true)),
            // The search keeps only the newest FIND_CAP, so a full list is a
            // floor, not a total: say so rather than claim an exact count.
            (false, n) => {
                let more = if n == crew_term::FIND_CAP { "+" } else { "" };
                Some((format!("{}/{n}{more}", self.sel + 1), false))
            }
        }
    }

    /// The row the bar sits on: the top, unless the current match is there
    /// (the oldest line of the buffer cannot be scrolled below row 0), when
    /// the bar steps down to the bottom row rather than cover what was found.
    pub fn bar_row(&self, off: usize, rows: u16) -> u16 {
        let on_top = self
            .matches
            .get(self.sel)
            .is_some_and(|m| (m.line..=m.end_line).any(|l| i64::from(l) + off as i64 == 0));
        if on_top {
            rows.saturating_sub(1)
        } else {
            0
        }
    }

    /// The viewport runs to tint, `off` lines scrolled back: every row but
    /// the bar's (see [`TermFind::bar_row`]).
    pub fn tints(&self, off: usize, cols: u16, rows: u16) -> Vec<Tint> {
        let (mut out, bar) = (Vec::new(), self.bar_row(off, rows));
        for (i, m) in self.matches.iter().enumerate() {
            for line in m.line..=m.end_line {
                let row = i64::from(line) + off as i64;
                if row < 0 || row >= i64::from(rows) || row == i64::from(bar) {
                    continue;
                }
                let from = if line == m.line { m.col } else { 0 };
                let to = if line == m.end_line {
                    m.end_col
                } else {
                    cols.saturating_sub(1)
                };
                out.push(Tint {
                    row: row as u16,
                    from,
                    to,
                    current: i == self.sel,
                });
            }
        }
        out
    }

    /// The display offset that shows the current match, or `None` when it is
    /// already on a visible row below the bar. Otherwise it is centred, within
    /// the `history` there is (at the buffer's top that leaves it on row 0).
    pub fn reveal(&self, off: usize, rows: u16, history: usize) -> Option<usize> {
        let m = self.matches.get(self.sel)?;
        let row = i64::from(m.line) + off as i64;
        if row >= 1 && row < i64::from(rows) {
            return None;
        }
        let want = (i64::from(rows / 2) - i64::from(m.line)).clamp(0, history as i64) as usize;
        // Already as far as the buffer goes: the bar steps aside instead.
        (want != off).then_some(want)
    }
}

/// Drain a terminal's output for the tick, and keep an open find bar's count
/// honest as it lands (the reader the poll loop calls). True when bytes came.
pub(crate) fn read(t: &mut crate::pane::TermPane) -> bool {
    let got = t.pty.try_read() > 0;
    if let Some(f) = t.find.as_mut() {
        f.dirty |= got;
        let due = f
            .last
            .is_none_or(|at| at.elapsed().as_millis() >= REFRESH_MS);
        if f.dirty && due {
            f.refresh(&t.pty);
        }
    }
    got
}

#[cfg(test)]
#[path = "termfind_tests.rs"]
mod tests;
