//! An `old` whose lines are the file's, each off by the same leading
//! whitespace, lands as the file has it.
//!
//! WHY: after "the same text is there with different whitespace" and the
//! file's rows to copy, qwen-max sent the lines back without their
//! indentation: 8 misses on 2026-10-23, every one a unique window off by one
//! amount (+4, +8, +12, +16), and a14off1 spent its last 3 rounds resending
//! two of them. Codex's `apply_patch` falls back from an exact match to a
//! trimmed one, and Aider re-indents a block "GPT often messes up leading
//! whitespace" on, uniformly. This is that, held as tight as it can be: only
//! after an exact miss, only one place in the file, only leading whitespace,
//! and the same shift on every line. Anything looser stays a refusal.

/// `old` as the file has it, and `new` shifted the same way, when `old` has
/// two or more lines and matches exactly one run of `body`'s lines once each
/// line's leading whitespace is set aside, every line off by the same
/// whitespace. `None` otherwise, or when a line of `new` would lose
/// whitespace it does not have.
pub(super) fn respace(body: &str, old: &str, new: &str) -> Option<(String, String)> {
    let want: Vec<&str> = old.strip_suffix('\n').unwrap_or(old).split('\n').collect();
    if want.iter().filter(|l| !l.trim().is_empty()).count() < 2 {
        return None;
    }
    // Each line of `body`, with the byte it starts at.
    let mut lines = Vec::new();
    let mut at = 0;
    for l in body.split_inclusive('\n') {
        lines.push((at, l.strip_suffix('\n').unwrap_or(l)));
        at += l.len();
    }
    let n = want.len();
    let fits = |i: usize| {
        (0..n).all(|k| match (lines.get(i + k), want[k].trim()) {
            (Some((_, l)), "") => l.trim().is_empty(),
            (Some((_, l)), _) => l.trim_start() == want[k].trim_start(),
            (None, _) => false,
        })
    };
    let mut hits = (0..lines.len()).filter(|&i| fits(i));
    let (i, None) = (hits.next()?, hits.next()) else {
        return None;
    };
    let shift = one_shift(&lines[i..i + n], &want)?;
    let end = lines[i + n - 1].0 + lines[i + n - 1].1.len();
    let end = if old.ends_with('\n') { end + 1 } else { end };
    let theirs = body.get(lines[i].0..end)?.to_string();
    Some((theirs, moved(new, &shift)?))
}

/// How the file's lines sit against `old`'s: more whitespace in front
/// (`Ok`) or less (`Err`), the same on every non-blank line, and not none.
fn one_shift(lines: &[(usize, &str)], want: &[&str]) -> Option<Result<String, String>> {
    let lead = |s: &str| s[..s.len() - s.trim_start().len()].to_string();
    let mut said: Option<Result<String, String>> = None;
    for ((_, l), w) in lines.iter().zip(want).filter(|(_, w)| !w.trim().is_empty()) {
        let (theirs, ours) = (lead(l), lead(w));
        let this = match (
            theirs.strip_suffix(ours.as_str()),
            ours.strip_suffix(theirs.as_str()),
        ) {
            (Some(more), _) if !more.is_empty() => Ok(more.to_string()),
            (_, Some(less)) if !less.is_empty() => Err(less.to_string()),
            _ => return None,
        };
        if said.as_ref().is_some_and(|s| *s != this) {
            return None;
        }
        said = Some(this);
    }
    said
}

/// `new` with the same whitespace added to, or taken off, the front of each
/// line that has text.
fn moved(new: &str, shift: &Result<String, String>) -> Option<String> {
    let line = |l: &str| match shift {
        _ if l.trim().is_empty() => Some(l.to_string()),
        Ok(more) => Some(format!("{more}{l}")),
        Err(less) => l.strip_prefix(less.as_str()).map(str::to_string),
    };
    let out: Option<Vec<String>> = new.split('\n').map(line).collect();
    Some(out?.join("\n"))
}

#[cfg(test)]
#[path = "editindent_tests.rs"]
mod tests;
