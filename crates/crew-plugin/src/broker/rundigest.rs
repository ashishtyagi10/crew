//! A cut build's errors, every one by where it is, at the top.
//!
//! WHY: [`super::runfit`] keeps a long run's tail, so a build with fifty
//! errors reached the agent as its last two. On the 2026-10-22/23 probes the
//! compile-error task's `cargo test` ran 54 times, cut every time (312 lines
//! gone), and the agent fixed the two it could see each round and ran it
//! again; no run passed. The whole output is saved to a file the result names,
//! and no agent read it. One line naming every error's place lets one round
//! fix them all, now that a round's edits all run.

/// Bytes the line may take before it says how many more it left out.
const MAX: usize = 400;

/// `N errors at path:line,line · path:line …`, from every error a compiler
/// wrote in `stdout` and `stderr` (rustc's `error…` then ` --> path:line:col`;
/// a note's or help's arrow is not an error's). Empty when there are fewer
/// than two, which the tail shows anyway.
pub(super) fn digest(stdout: &str, stderr: &str) -> String {
    let mut files: Vec<(&str, Vec<&str>)> = Vec::new();
    let (mut error, mut n) = (false, 0);
    for line in stderr.lines().chain(stdout.lines()) {
        let t = line.trim_start();
        if t.starts_with("error") || t.starts_with("warning") {
            error = t.starts_with("error");
            continue;
        }
        let Some(at) = t.strip_prefix("--> ").filter(|_| error) else {
            continue;
        };
        error = false;
        let mut parts = at.trim().rsplitn(3, ':');
        let (_col, line, path) = (parts.next(), parts.next(), parts.next());
        let (Some(line), Some(path)) = (line, path) else {
            continue;
        };
        n += 1;
        match files.iter_mut().find(|(p, _)| *p == path) {
            Some((_, lines)) => lines.push(line),
            None => files.push((path, vec![line])),
        }
    }
    if n < 2 {
        return String::new();
    }
    let mut out = format!("{n} errors at");
    let mut left = 0;
    for (k, (path, lines)) in files.iter().enumerate() {
        let one = format!(
            "{} {path}:{}",
            if k == 0 { "" } else { " \u{b7}" },
            lines.join(",")
        );
        if out.len() + one.len() > MAX {
            left = files.len() - k;
            break;
        }
        out.push_str(&one);
    }
    if left > 0 {
        out.push_str(&format!(
            " \u{b7} and {left} more file{}",
            if left == 1 { "" } else { "s" }
        ));
    }
    out + "\n"
}

/// Whether a `sys:run` result is a command that ran and failed: its first
/// line is `exit N` with N not 0 (-1 is a signal). `sysrun` returns such a
/// run as `Ok`, because the output IS the answer; this is what keeps the
/// card from reading ✓ and the provider from being told `is_error: false`
/// about a build that did not build — the way Codex reports a non-zero exit.
/// A timeout is already an `Err`.
pub(crate) fn failed(text: &str) -> bool {
    text.lines()
        .next()
        .and_then(|l| l.strip_prefix("exit "))
        .and_then(|n| n.trim().parse::<i32>().ok())
        .is_some_and(|n| n != 0)
}

#[cfg(test)]
#[path = "rundigest_tests.rs"]
mod tests;
