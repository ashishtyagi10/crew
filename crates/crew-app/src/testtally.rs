//! What a test run's output says it ran: how many passed, how many failed,
//! and which.
//!
//! The proof under a turn (`chatproof`) said "`cargo test` passed" from the
//! exit code alone. That is an adjective, not evidence: a `cargo test
//! some_filter` that matched nothing exits 0 having run no test at all, and
//! read as checked. The runners print their own count at the end, and this
//! reads it — cargo's `test result:` lines (summed over every test binary),
//! pytest's closing `== … ==` line, jest's and vitest's `Tests` line. Output
//! nothing here recognises is `None`, and the proof says what it said before.

/// The counts one run reported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Tally {
    pub passed: u32,
    pub failed: u32,
    /// The failed tests by name, where the runner names them per line.
    pub names: Vec<String>,
}

impl Tally {
    pub(crate) fn ran(&self) -> u32 {
        self.passed + self.failed
    }
}

/// The tally `out` reports, if it holds a runner's summary.
pub(crate) fn tally(out: &str) -> Option<Tally> {
    cargo(out).or_else(|| pytest(out)).or_else(|| jest(out))
}

/// `test result: ok. 41 passed; 1 failed; …` once per test binary.
fn cargo(out: &str) -> Option<Tally> {
    let mut t: Option<Tally> = None;
    for line in out.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("test result: ") {
            let sum = t.get_or_insert_with(Tally::default);
            sum.passed += count(rest, "passed");
            sum.failed += count(rest, "failed");
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|l| l.strip_suffix(" ... FAILED"))
        {
            t.get_or_insert_with(Tally::default)
                .names
                .push(name.to_string());
        }
    }
    t.filter(|t| t.ran() > 0 || out.contains("test result: "))
}

/// `===== 2 failed, 40 passed in 3.10s =====`, or `no tests ran`.
fn pytest(out: &str) -> Option<Tally> {
    let line = out
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with("==") && l.ends_with("==") && l.contains(" in "))?;
    let said = line.trim_matches('=');
    if !said.contains("passed") && !said.contains("failed") && !said.contains("no tests ran") {
        return None;
    }
    Some(Tally {
        passed: count(said, "passed"),
        failed: count(said, "failed") + count(said, "error") + count(said, "errors"),
        names: Vec::new(),
    })
}

/// jest's `Tests:       2 failed, 40 passed, 42 total` and vitest's
/// `Tests  2 failed | 40 passed (42)`.
fn jest(out: &str) -> Option<Tally> {
    let line = out
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with("Tests:") || l.starts_with("Tests "))?;
    let said = line.trim_start_matches("Tests").trim_start_matches(':');
    (said.contains("passed") || said.contains("failed")).then(|| Tally {
        passed: count(said, "passed"),
        failed: count(said, "failed"),
        names: Vec::new(),
    })
}

/// The number written just before `word` in `s` (`"41 passed"` → 41), or 0.
fn count(s: &str, word: &str) -> u32 {
    let words: Vec<&str> = s
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '|' | '(' | ')'))
        .filter(|w| !w.is_empty())
        .collect();
    words
        .windows(2)
        .filter(|w| w[1].trim_end_matches('.') == word)
        .filter_map(|w| w[0].parse::<u32>().ok())
        .sum()
}

#[cfg(test)]
#[path = "testtally_tests.rs"]
mod tests;
