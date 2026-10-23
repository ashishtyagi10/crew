//! What a missed `old` is when it is not one retyped line: `new` already in
//! the file, `old` gluing two places together, or `old` parting from the file
//! partway down.
//!
//! WHY: a miss that started right was told only "the rest of `old` does not
//! match what follows it", with no line and nothing to copy, whatever the
//! shape. On the 2026-10-23 a14 probes misses cost 14 of 60 rounds. One
//! 6-line `old` went out five rounds running: its fifth line was
//! `(hi + 0.05) / (lo + 0.05)`, where the file's line 169 read
//! `((hi + 0.05) / (lo + 0.05)) as f64`. Another glued line 126 to line 159,
//! and two redid an edit already made, sent there by "the nearest is line
//! 159; copy its text". Aider answers these with "did you mean to match some
//! of these actual lines" and "the REPLACE lines are already in the file".
//! So does this.
use super::editnear::rows;

/// Shortest `new`, trimmed, worth looking for: a shorter one is in a file by
/// chance.
const ALREADY: usize = 8;

/// At most this many rows are shown, and this many places named.
const SHOWN: usize = 4;

/// `new` is already in `body`, at most [`SHOWN`] times: the edit was
/// probably made already, and copying the nearest line would redo it. Not
/// when `old` holds `new` (a deletion), since then `new` is there either way.
pub(super) fn already(body: &str, old: &str, new: &str) -> Option<String> {
    let want = new.trim();
    if want.chars().count() < ALREADY || old.contains(want) {
        return None;
    }
    let ats: Vec<usize> = body.match_indices(want).map(|(at, _)| at).collect();
    if ats.is_empty() || ats.len() > SHOWN {
        return None;
    }
    let line = |at: usize| body[..at].matches('\n').count();
    let lines: Vec<&str> = body.lines().collect();
    let first = line(ats[0]);
    let end = (first + want.lines().count().clamp(1, SHOWN)).min(lines.len());
    let named: Vec<String> = ats.iter().map(|&at| (line(at) + 1).to_string()).collect();
    Some(format!(
        "`old` is not in the file, but `new` already is ({} {}) \u{2014} the edit may \
         have been made already; check it before sending another:\n{}",
        if named.len() == 1 { "line" } else { "lines" },
        named.join(", "),
        rows(&lines, first, end)
    ))
}

/// `old` is in the file in pieces, each in one place, but not together: two
/// places sent as one `old`.
pub(super) fn apart(body: &str, old: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();
    let (_, found, whole) = pieces(&lines, old);
    if !whole || found.len() < 2 || found.iter().any(|p| !p.once) {
        return None;
    }
    let said: Vec<String> = (found.iter().take(SHOWN))
        .map(|p| {
            let are = if p.len == 1 { "is" } else { "are" };
            format!("its {} {are} {}", span(p.of, p.len), span(p.at, p.len))
        })
        .collect();
    Some(format!(
        "`old` is not in the file as one piece, but it is there in {} pieces \u{2014} {} \u{2014} \
         send one edit per place (several go in one call as \
         \"edits\": [{{\"old\": \u{2026}, \"new\": \u{2026}}}, \u{2026}])",
        found.len(),
        said.join("; ")
    ))
}

/// `old` starts as the file does and parts from it: where, and the file's
/// lines from there to copy.
pub(super) fn parted(body: &str, old: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();
    let (want, found, _) = pieces(&lines, old);
    let p = found.first().filter(|p| p.of == 0 && p.len < want.len())?;
    let (at, theirs) = (p.at + p.len, quote(want[p.len]));
    let first = match p.len {
        1 => "line is".to_string(),
        n => format!("{n} lines are"),
    };
    let head = format!(
        "`old` is not in the file: its first {first} {}, then it parts",
        span(p.at, p.len)
    );
    if at >= lines.len() {
        return Some(format!("{head} where the file ends and `old` has {theirs}"));
    }
    let end = (at + (want.len() - p.len).min(SHOWN)).min(lines.len());
    Some(format!(
        "{head} \u{2014} where `old` has {theirs}, line {} reads (copy it, not the \
         numbers):\n{}",
        at + 1,
        rows(&lines, at, end)
    ))
}

/// A run of `old`'s lines found together in the file.
struct Piece {
    /// Where in `old` it starts, and in the file, and how many lines.
    of: usize,
    at: usize,
    len: usize,
    /// No other place in the file holds as long a run from `of`.
    once: bool,
}

/// `old`'s lines trimmed, the runs of them found in `lines` from the top,
/// each the longest from where the last stopped, and whether they reach the
/// end. A blank line between runs belongs to none.
fn pieces<'a>(lines: &[&str], old: &'a str) -> (Vec<&'a str>, Vec<Piece>, bool) {
    let want: Vec<&str> = old.trim_matches('\n').lines().map(str::trim).collect();
    let n = want.len();
    // `old` may start partway into its first line and stop partway into its last.
    let fits = |j: usize, line: &str| match line.trim() {
        l if j == 0 && n > 1 => l.ends_with(want[0]),
        l if j == n - 1 && n > 1 => l.starts_with(want[j]),
        l => l == want[j],
    };
    let run = |j: usize, i: usize| {
        (0..n - j)
            .take_while(|&k| lines.get(i + k).is_some_and(|l| fits(j + k, l)))
            .count()
    };
    let (mut found, mut j) = (Vec::new(), 0);
    while j < n {
        if want[j].is_empty() && j > 0 {
            j += 1;
            continue;
        }
        let runs: Vec<usize> = (0..lines.len()).map(|i| run(j, i)).collect();
        let len = runs.iter().copied().max().unwrap_or(0);
        let Some(at) = runs.iter().position(|&k| k == len).filter(|_| len > 0) else {
            return (want, found, false);
        };
        let once = runs.iter().filter(|&&k| k == len).count() == 1;
        found.push(Piece {
            of: j,
            at,
            len,
            once,
        });
        j += len;
    }
    (want, found, true)
}

/// `line 5` or `lines 5–9`, counted from 1.
fn span(at: usize, len: usize) -> String {
    match len {
        1 => format!("line {}", at + 1),
        _ => format!("lines {}\u{2013}{}", at + 1, at + len),
    }
}

/// A line of `old` short enough to sit inside an error message.
fn quote(s: &str) -> String {
    const CAP: usize = 48;
    match s.chars().count() {
        0 => "a blank line".into(),
        n if n <= CAP => format!("\u{201c}{s}\u{201d}"),
        _ => format!(
            "\u{201c}{}\u{2026}\u{201d}",
            s.chars().take(CAP).collect::<String>()
        ),
    }
}

#[cfg(test)]
#[path = "editmiss_tests.rs"]
mod tests;
