//! `sys:edit {"line": N}` — which of several identical matches to change.
//!
//! WHY: `old` appearing twice was refused (rightly: editing the first is a
//! guess), but the refusal never said WHERE the two were, and a model had no
//! way to name one but to quote more lines around it. Models asked for the
//! obvious thing on their own: qwen-max sent `"line": 131` with an edit thirty
//! times across two probe runs, unprompted, and every one was dropped unread,
//! so the same failing edit went out six times in one turn. Codex's
//! `apply_patch` anchors a hunk by a context line for the same reason, and
//! crew's own reads number every line (`  41│`), so the number is to hand.
//!
//! The match stays exact. A line only chooses AMONG exact matches, and only
//! one whose lines contain it, so a stale number errors rather than editing
//! the wrong copy.

/// Where every occurrence of `old` is, and the line each starts on.
pub(super) fn lines_of(body: &str, ats: &[usize]) -> Vec<usize> {
    ats.iter().map(|&at| line_of(body, at)).collect()
}

/// The one occurrence (by byte offset in `ats`) whose lines hold `line`.
pub(super) fn choose(body: &str, old: &str, ats: &[usize], line: usize) -> Result<usize, String> {
    let span = old.trim_end_matches('\n').matches('\n').count();
    let firsts = lines_of(body, ats);
    let hits: Vec<usize> = (0..ats.len())
        .filter(|&i| (firsts[i]..=firsts[i] + span).contains(&line))
        .collect();
    match hits[..] {
        [i] => Ok(ats[i]),
        [] => Err(format!(
            "`old` appears {} times ({}), and none of them is on line {line} \u{2014} pass a \
             \"line\" inside one of them",
            ats.len(),
            at_lines(&firsts)
        )),
        _ => Err(format!(
            "`old` appears {} times on line {line} itself \u{2014} include more of that line \
             to name just one",
            hits.len()
        )),
    }
}

/// The refusal for an `old` found more than once, saying where each is.
pub(super) fn ambiguous(body: &str, ats: &[usize]) -> String {
    format!(
        "`old` appears {} times ({}), so there is no single place to put `new` \u{2014} pass \
         \"line\" with the number of the one to change, or include more of the surrounding \
         lines to name just one; \"all\": true replaces every one (a rename)",
        ats.len(),
        at_lines(&lines_of(body, ats))
    )
}

/// `lines 84 and 131`, or the first few of many.
fn at_lines(firsts: &[usize]) -> String {
    const SHOWN: usize = 6;
    let mut words: Vec<String> = firsts.iter().take(SHOWN).map(|l| l.to_string()).collect();
    match firsts.len() {
        1 => format!("line {}", words[0]),
        n if n > SHOWN => format!("lines {}, \u{2026}", words.join(", ")),
        _ => {
            let last = words.pop().unwrap_or_default();
            format!("lines {} and {last}", words.join(", "))
        }
    }
}

/// How the lines of the file as the call found it moved while a batch's
/// earlier edits landed, so every `line` in one call means the file the model
/// read, not the file half-way through the batch. Without it an edit that
/// added two lines above would point the next edit's `line` at the copy two
/// lines down.
#[derive(Default)]
pub(super) struct Shift(Vec<Vec<(usize, usize, isize)>>);

impl Shift {
    /// Note that `old` at each of `ats` in `before` became `new`.
    pub(super) fn note(&mut self, before: &str, ats: &[usize], old: &str, new: &str) {
        let (was, now) = (old.matches('\n').count(), new.matches('\n').count());
        let moved = lines_of(before, ats)
            .into_iter()
            .map(|first| (first, was, now as isize - was as isize))
            .collect();
        self.0.push(moved);
    }

    /// Where `line` of the file as it was read sits now.
    pub(super) fn now(&self, mut line: usize) -> usize {
        for moved in &self.0 {
            // Each step's offsets are in the numbering before it landed.
            let at = line;
            for &(first, was, d) in moved {
                if at >= first + was {
                    line = line.saturating_add_signed(d);
                }
            }
        }
        line
    }
}

/// The line byte `at` is on, counted the way the numbered reads count.
fn line_of(s: &str, at: usize) -> usize {
    1 + s.as_bytes()[..at].iter().filter(|&&b| b == b'\n').count()
}

#[cfg(test)]
#[path = "editpick_tests.rs"]
mod tests;
