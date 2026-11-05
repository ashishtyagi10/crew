//! Red, then green: whether a turn reproduced what it fixed.
//!
//! A senior engineer runs the failing test before touching the code, so the
//! fix is proven by the same run turning green, not just by a green run.
//! `chatproof` keeps only the last run after the last edit, so a red run
//! before the first edit was dropped and the pane could not tell a fix that
//! was reproduced from one that was only checked. This keeps that red run,
//! and when the run after the edit passes and covers it, the verdict names
//! both: `✓ fixed: … — 2 of 44 failed before the fix; 44 pass (12s)`,
//! short enough to stay one row where `✓ checked` does. A turn that never
//! reproduced keeps `✓ checked`.
use crate::testtally::Tally;

/// The last failing build/test heard before the turn's first edit, with
/// what its output counted.
#[derive(Clone, Debug, Default)]
pub(crate) struct Repro(Option<(String, Option<Tally>)>);

impl Repro {
    /// A run ended before any edit: a red one is the reproduction, and the
    /// same command going green again (a flaky test) takes it back.
    pub(crate) fn before_edit(&mut self, cmd: &str, ok: bool, t: Option<&Tally>) {
        match ok {
            false => self.0 = Some((cmd.to_string(), t.cloned())),
            true if self.0.as_ref().is_some_and(|(was, _)| was == cmd) => self.0 = None,
            true => {}
        }
    }

    /// The red → green verdict when the passing run after the last edit
    /// covers the reproduction: the same command, or a broader one whose
    /// words lead it (`cargo test -p x` covers `cargo test -p x name`). A
    /// pass that counted no tests proves nothing, so it stays `None`.
    pub(crate) fn fixed(&self, after: &str, t: Option<&Tally>, ms: u64) -> Option<String> {
        let (before, red) = self.0.as_ref()?;
        if t.is_some_and(|t| t.ran() == 0 || t.failed > 0) || !covers(after, before) {
            return None;
        }
        let was = match red.as_ref().filter(|r| r.failed > 0) {
            Some(r) => format!("{} of {} failed", r.failed, r.ran()),
            None => "failed".into(),
        };
        let took = (ms >= 1000).then(|| crate::chatswarmspan::fmt_ms(ms));
        let n = t.map(|t| t.ran());
        Some(match (before == after, n) {
            (true, Some(n)) => {
                let took = took.map(|s| format!(" ({s})")).unwrap_or_default();
                format!("\u{2713} fixed: `{after}` \u{2014} {was} before the fix; {n} pass{took}")
            }
            (true, None) => format!("\u{2713} fixed: `{after}` {was} before the fix; passes now"),
            (false, n) => {
                let took = took.map(|s| format!(", {s}"));
                let now = match n {
                    Some(n) => format!(" ({}{})", tests(n), took.unwrap_or_default()),
                    None => String::new(),
                };
                format!("\u{2713} fixed: `{before}` {was} before the fix; `{after}` passes{now}")
            }
        })
    }
}

fn tests(n: u32) -> String {
    format!("{n} test{}", if n == 1 { "" } else { "s" })
}

/// `after`'s words are the first words of `before`: the same run or a
/// wider one over the same target.
fn covers(after: &str, before: &str) -> bool {
    let (a, b): (Vec<&str>, Vec<&str>) = (
        after.split_whitespace().collect(),
        before.split_whitespace().collect(),
    );
    !a.is_empty() && b.starts_with(&a)
}

#[cfg(test)]
#[path = "chatrepro_tests.rs"]
mod tests;
