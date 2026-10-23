//! Where a missed `old` probably was: the lines nearest it, numbered as a
//! read numbers them.
//!
//! WHY: most edits that miss were never in the file as written: the model
//! retyped the line it meant from memory (`let mut free = Vec::new();` for
//! `let mut free: Vec<f32> = Vec::new();`). 31 of 36 "not in the file"
//! misses on the 2026-10-22/23 probes were that, and the refusal said only
//! "read it", which costs a round, or the model guessed again. Aider answers
//! the same miss with "did you mean to match some of these actual lines",
//! and that is what this gives: the line to copy, where it is.

/// Lines longer than this are compared and shown by their start.
const CAP: usize = 160;

/// How alike a line must be to `old`'s first to be offered: below it the
/// "nearest" line is as likely to mislead as to help.
const NEAR: f32 = 0.6;

/// At most this many lines are shown, from the nearest one down.
const SHOWN: usize = 4;

/// The lines of `body` nearest `old`, numbered, as many as `old` has (up to
/// [`SHOWN`]), when its first line has a near enough match.
pub(super) fn nearest(body: &str, old: &str) -> Option<String> {
    let first = old.lines().map(str::trim).find(|l| !l.is_empty())?;
    let lines: Vec<&str> = body.lines().collect();
    let (at, score) = (lines.iter().enumerate())
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, ratio(first, l.trim())))
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    if score < NEAR {
        return None;
    }
    let span = old.trim().lines().count().clamp(1, SHOWN);
    Some(rows(&lines, at, (at + span).min(lines.len())))
}

/// The lines of `body` that hold `old` once its whitespace is ignored,
/// numbered: the "different whitespace" miss said only to read the file, and
/// was 14 of the probes' 69 failed edits. Shown, the lines can be copied as
/// they are, indentation and all.
pub(super) fn spaced(body: &str, old: &str) -> Option<String> {
    let want: String = old.split_whitespace().collect();
    // Each non-space char of `body`, with the line it is on.
    let (flat, line): (String, Vec<usize>) = (body.lines().enumerate())
        .flat_map(|(i, l)| {
            l.chars()
                .filter(|c| !c.is_whitespace())
                .map(move |c| (c, i))
        })
        .unzip();
    let start = flat.find(&want)?;
    let (first, chars) = (flat[..start].chars().count(), want.chars().count());
    let (a, b) = (line[first], line[first + chars.max(1) - 1]);
    let lines: Vec<&str> = body.lines().collect();
    Some(rows(&lines, a, (b + 1).min(a + SHOWN)))
}

/// `lines[at..end]`, each under its number as a read shows it.
pub(super) fn rows(lines: &[&str], at: usize, end: usize) -> String {
    let w = end.to_string().len();
    let row = |(k, l): (usize, &&str)| {
        let text: String = l.chars().take(CAP).collect();
        let cut = if l.chars().count() > CAP {
            "\u{2026}"
        } else {
            ""
        };
        format!("{:>w$}\u{2502} {text}{cut}", at + k + 1)
    };
    let shown: Vec<String> = lines[at..end].iter().enumerate().map(row).collect();
    shown.join("\n")
}

/// How alike two lines are, 0 to 1: twice their longest common subsequence
/// over their total length, as difflib's ratio measures it.
fn ratio(a: &str, b: &str) -> f32 {
    let a: Vec<char> = a.chars().take(CAP).collect();
    let b: Vec<char> = b.chars().take(CAP).collect();
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut prev = vec![0u16; b.len() + 1];
    for &x in &a {
        let mut cur = vec![0u16; b.len() + 1];
        for (j, &y) in b.iter().enumerate() {
            cur[j + 1] = if x == y {
                prev[j] + 1
            } else {
                cur[j].max(prev[j + 1])
            };
        }
        prev = cur;
    }
    2.0 * f32::from(prev[b.len()]) / (a.len() + b.len()) as f32
}

#[cfg(test)]
#[path = "editnear_tests.rs"]
mod tests;
