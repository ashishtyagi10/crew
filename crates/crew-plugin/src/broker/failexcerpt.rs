//! The part of a failing check worth reading.
//!
//! A build or a test run prints its progress first and its verdict last:
//! `Compiling …` forty times, `running 512 tests`, a line per passing test,
//! and only then the `error[E0308]`, the failing test's name and its
//! `left`/`right`. Both readers of a failed check took the TOP of that (the
//! pane's verdict its first six lines, the repair pass its first forty), so
//! the agent asked to fix a build was usually handed a screen of `Compiling`
//! and nothing it could act on.
//!
//! So lines are picked by what they say, not by where they are: the lines
//! that name a failure, each with the two after it (rustc's ` --> file:line`,
//! an assertion's `left:` and `right:`), then the end of the output, where
//! every test runner prints its summary. Progress and passing tests never
//! count as failure lines, whatever their names say: a suite has tests called
//! `a_failed_check_is_reported`, and forty of those would fill the budget.

/// Lines kept after each failure line.
const CONTEXT: usize = 2;
/// Failure lines and their context, at most. Past this it is the same
/// failure again, or a cascade from the first.
const PICKED: usize = 60;
/// Lines from the end, where test runners print the summary.
const TAIL: usize = 15;
/// When nothing names a failure: this many from the end, not the start.
const FALLBACK: usize = 40;
/// Written where lines were skipped, so a reader never takes two distant
/// lines for neighbours.
const GAP: &str = "\u{2026}";

/// Words that mark a failure line, matched lowercase anywhere in it. `fail`
/// on its own is matched as a word (`FAIL src/a.test.js`, `--- FAIL:`), so
/// `test_fail_path` stays a name.
const MARKS: &str = "error failed panicked assert expected found traceback exception \u{2717}";

/// Cargo's progress verbs. A crate called `thiserror` compiling is not an
/// error, and forty of these are what used to fill the brief.
const PROGRESS: &str = "Compiling Checking Building Downloading Downloaded Fresh Documenting \
                        Updating Locking Blocking Fetching Finished Running Doc-tests";

/// What the repair pass is shown of a failed check: the `exit N` line, the
/// failure lines with their context, then the last lines, with `…` wherever
/// something was skipped. Nothing that names a failure: the end instead.
pub(crate) fn excerpt(text: &str) -> String {
    let (exit, lines) = split(text);
    let useful = useful(&lines);
    let picks = picks(&lines, &useful);
    let mut keep = vec![false; lines.len()];
    let mut n = 0;
    'pick: for &p in &picks {
        for &i in useful[p..].iter().take(CONTEXT + 1) {
            if n == PICKED {
                break 'pick;
            }
            if !keep[i] {
                keep[i] = true;
                n += 1;
            }
        }
    }
    let end = if picks.is_empty() { FALLBACK } else { TAIL };
    useful.iter().rev().take(end).for_each(|&i| keep[i] = true);
    let mut out: Vec<&str> = exit.into_iter().collect();
    let mut skipped = false;
    for (line, kept) in lines.iter().zip(&keep) {
        if *kept {
            if std::mem::take(&mut skipped) {
                out.push(GAP);
            }
            out.push(line);
        } else {
            skipped |= !line.trim().is_empty();
        }
    }
    if skipped {
        out.push(GAP);
    }
    out.join("\n")
}

/// The pane's verdict on a failed check, in at most `max` lines: the first
/// failure lines (each with rustc's `-->` location when one follows it), and
/// the summary last, `test result: FAILED. 39 passed; 1 failed` when cargo
/// printed one, otherwise the last failure line. Nothing that names a
/// failure: the last lines.
pub(crate) fn headline(text: &str, max: usize) -> Vec<&str> {
    let (_, lines) = split(text);
    let useful = useful(&lines);
    let picks = picks(&lines, &useful);
    let Some(&last) = picks.last() else {
        let from = useful.len().saturating_sub(max);
        return useful[from..].iter().map(|&i| lines[i]).collect();
    };
    let summary = picks
        .iter()
        .rev()
        .copied()
        .find(|&p| lines[useful[p]].trim_start().starts_with("test result:"))
        .unwrap_or(last);
    let at = |p: usize| useful.get(p).map_or("", |&i| lines[i].trim_start());
    let room = max.saturating_sub(1);
    let mut shown: Vec<usize> = Vec::new();
    for &p in &picks {
        let place = at(p + 1).starts_with("-->").then_some(p + 1);
        for q in [Some(p), place].into_iter().flatten() {
            if shown.len() < room && q != summary && !shown.contains(&q) {
                shown.push(q);
            }
        }
    }
    if max > 0 {
        shown.push(summary);
    }
    shown.into_iter().map(|p| lines[useful[p]]).collect()
}

/// The `exit N` line `sys:run` opens with, and the lines after it.
pub(crate) fn split(text: &str) -> (Option<&str>, Vec<&str>) {
    let mut lines: Vec<&str> = text.lines().collect();
    let exit = lines.first().copied().filter(|l| {
        l.strip_prefix("exit ")
            .is_some_and(|n| n.trim().parse::<i32>().is_ok())
    });
    if exit.is_some() {
        lines.remove(0);
    }
    (exit, lines)
}

/// Indices of the lines worth reading at all: not blank, not progress. An
/// output that is ALL progress keeps its non-blank lines, so it still says
/// something.
pub(crate) fn useful(lines: &[&str]) -> Vec<usize> {
    let real = |i: &usize| !lines[*i].trim().is_empty();
    let quiet: Vec<usize> = (0..lines.len())
        .filter(|i| real(i) && !progress(lines[*i]))
        .collect();
    match quiet.is_empty() {
        true => (0..lines.len()).filter(real).collect(),
        false => quiet,
    }
}

/// Positions in `useful` of the failure lines, or of the warnings when
/// nothing failed outright.
pub(crate) fn picks(lines: &[&str], useful: &[usize]) -> Vec<usize> {
    let find = |f: fn(&str) -> bool| -> Vec<usize> {
        (0..useful.len()).filter(|&p| f(lines[useful[p]])).collect()
    };
    let errors = find(failure);
    match errors.is_empty() {
        true => find(|l| l.to_lowercase().contains("warning:")),
        false => errors,
    }
}

fn progress(line: &str) -> bool {
    let t = line.trim_start();
    PROGRESS
        .split_whitespace()
        .any(|v| t.strip_prefix(v).is_some_and(|r| r.starts_with(' ')))
}

/// A line that names a failure, and is neither a warning that mentions one
/// nor a passing test whose name does.
fn failure(line: &str) -> bool {
    let t = line.trim().to_lowercase();
    if t.starts_with("warning") || passing(&t) {
        return false;
    }
    MARKS.split_whitespace().any(|m| t.contains(m)) || word(&t, "fail")
}

/// A passing test or a green summary, lowercase: cargo, pytest -v, jest,
/// mocha and go each print one per test.
fn passing(t: &str) -> bool {
    t.ends_with("... ok")
        || t.contains("... ignored")
        || t.starts_with("test result: ok")
        || t.starts_with(['\u{2713}', '\u{2714}'])
        || t.starts_with("pass ")
        || t.starts_with("--- pass")
        || t.starts_with("ok ")
        || (t.contains("::") && t.contains(" passed"))
}

/// `w` in `t` with no letter, digit or `_` on either side.
fn word(t: &str, w: &str) -> bool {
    let edge = |c: Option<char>| !c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    t.match_indices(w)
        .any(|(i, _)| edge(t[..i].chars().next_back()) && edge(t[i + w.len()..].chars().next()))
}

#[cfg(test)]
#[path = "failexcerpt_tests.rs"]
mod tests;
