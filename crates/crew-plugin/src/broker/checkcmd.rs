//! The check command a repo already wrote down.
//!
//! WHY: `.crew/check` is declared rather than guessed, and the reason is good
//! — the command spends your CPU and can take minutes, so a tool that starts
//! doing that unasked is a tool people turn off. But a repo whose `AGENTS.md`
//! says *"run `cargo test --workspace` before pushing"* HAS declared it. It
//! declared it to every contributor, in the file that exists so nobody has to
//! be told twice, and crew has been reading that file in front of every task
//! since 0.22.26 while ignoring the one line in it that is executable.
//!
//! So this is not guessing, and the distinction is the whole design: nothing
//! here infers a command from the SHAPE of a project (a `Cargo.toml` still
//! only earns the one-time tip). It reads a command the project wrote, in
//! prose it wrote, and it is conservative about what it will believe:
//!
//! - the first word must be a known runner, so an instruction file can never
//!   talk crew into running something that is not a build tool;
//! - shell chaining, redirection and substitution are refused outright — a
//!   project that needs `a && b` puts it in `.crew/check`, where a human on
//!   this machine typed it;
//! - the line has to be ABOUT checking, so a `git clone` in a setup section
//!   is not mistaken for a test command.
//!
//! And whatever it finds is named out loud in the verdict, because a command
//! the user did not type must never be a command the user cannot trace.

/// Where a check command came from, so the verdict can say.
pub(crate) struct Declared {
    pub cmd: String,
    /// The instruction file it was read out of, or `None` for `.crew/check`
    /// — which the user typed on this machine and so needs no attribution.
    pub from: Option<String>,
}

/// The check command for `base`: what the user declared, else what the repo
/// declared to every contributor.
///
/// `.crew/check` wins whenever it exists. It is the more specific answer and
/// the one a human here typed, so a project whose `AGENTS.md` names a
/// twenty-minute suite can still be pointed at something quicker.
pub(crate) fn declared_at(base: &std::path::Path) -> Option<Declared> {
    if let Some(cmd) = super::selfcheck::command_at(base) {
        return Some(Declared { cmd, from: None });
    }
    if !enabled() {
        return None;
    }
    let (text, names) = super::agentsmd::block_at(base)?;
    let cmd = from_instructions(&text)?;
    Some(Declared {
        cmd,
        from: names.first().cloned(),
    })
}

/// `CREW_CHECK=0` turns the check off entirely.
///
/// It exists because of this file. While the only command was one you wrote
/// into `.crew/check`, deleting the file WAS the off switch; now that a repo
/// can supply one, someone who never asked for a check needs a way to say so
/// that does not mean editing a file the repo owns.
pub(crate) fn enabled() -> bool {
    std::env::var("CREW_CHECK").ok().as_deref() != Some("0")
}

/// Commands crew will believe are a check. Everything here builds, tests or
/// lints; nothing here moves files, talks to a network or installs anything.
const RUNNERS: &[&str] = &[
    "cargo", "npm", "pnpm", "yarn", "bun", "deno", "make", "just", "pytest", "tox", "go", "mvn",
    "gradle", "dotnet", "swift", "zig", "rake", "bundle", "ninja", "ctest", "mix", "stack",
];

/// What a line has to be about, best reason first — a file that names both a
/// test and a build command should have its TEST command run.
const REASONS: &[&str] = &["test", "check", "verify", "lint", "build"];

/// Shell syntax that turns one command into several. Refused rather than
/// escaped: the point is that crew runs a build tool, not a script.
const CHAINING: &[&str] = &["&&", "||", ";", "|", ">", "<", "$(", "`", "\n"];

/// The check command `text` declares, if it declares one.
pub(crate) fn from_instructions(text: &str) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    let lines: Vec<&str> = text.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let Some(rank) = reason(line) else { continue };
        // Kept only if it beats what we have: an earlier line with a better
        // reason wins, and ties go to whichever came first.
        if best.as_ref().is_some_and(|(r, _)| *r <= rank) {
            continue;
        }
        let Some(cmd) = inline(line).or_else(|| fenced(&lines, i)) else {
            continue;
        };
        best = Some((rank, cmd));
    }
    best.map(|(_, c)| c)
}

/// How good a reason this line is to believe the command on it, or `None`
/// when the line is not about checking anything.
fn reason(line: &str) -> Option<usize> {
    let lower = line.to_ascii_lowercase();
    REASONS.iter().position(|w| lower.contains(w))
}

/// A command in backticks on this line.
fn inline(line: &str) -> Option<String> {
    let mut parts = line.split('`');
    parts.next()?; // before the first backtick
    parts.find_map(accept)
}

/// A command in the fenced block that opens within two lines of `i`.
///
/// Two, so that `Testing:` followed by a blank line and a fence still counts,
/// while a fence further down the file belongs to whatever introduced it.
fn fenced(lines: &[&str], i: usize) -> Option<String> {
    let open = (i + 1..=(i + 2).min(lines.len().saturating_sub(1)))
        .find(|&j| lines[j].trim_start().starts_with("```"))?;
    lines
        .get(open + 1..)?
        .iter()
        .take_while(|l| !l.trim_start().starts_with("```"))
        .find_map(|l| accept(l))
}

/// `s` as a command, if crew will run it.
fn accept(s: &str) -> Option<String> {
    let cmd = s.trim().trim_start_matches("$ ").trim();
    if cmd.is_empty() || CHAINING.iter().any(|c| cmd.contains(c)) {
        return None;
    }
    let first = cmd.split_whitespace().next()?;
    RUNNERS.contains(&first).then(|| cmd.to_string())
}

#[cfg(test)]
#[path = "checkcmd_tests.rs"]
mod tests;
