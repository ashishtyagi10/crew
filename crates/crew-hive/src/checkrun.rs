//! Whether a shell command is a check: a build, a test or a lint run.
//!
//! WHY here: the broker already keeps the list of runners it will believe a
//! check starts with (`checkcmd`, for the command a repo declares), and the
//! advisor now needs the same answer for a command the agent ran — "did a
//! check pass after the last edit?" (`advisor::done`). One list, in the crate
//! both can reach, so the two can never disagree about what a check is.
//!
//! Strict on purpose, since a check counted as passed turns the advisor's
//! read off: the first word must be a runner and the command must say what
//! it is for (`test`, `check`, `verify`, `lint`, `build`), so `cargo add` or
//! `npm install` is no check. And it must stand alone or be joined by `&&`:
//! `cargo test | tail` or `cargo test; echo` exits with the last command's
//! status, so its `exit 0` says nothing about the tests.

/// Commands crew will believe are a check. Everything here builds, tests or
/// lints; nothing here moves files, talks to a network or installs anything
/// on its own.
pub const RUNNERS: &[&str] = &[
    "cargo", "npm", "pnpm", "yarn", "bun", "deno", "make", "just", "pytest", "tox", "go", "mvn",
    "gradle", "dotnet", "swift", "zig", "rake", "bundle", "ninja", "ctest", "mix", "stack",
];

/// What a check command has to be about, best reason first: a file that
/// names both a test and a build command should have its TEST command run.
pub const REASONS: &[&str] = &["test", "check", "verify", "lint", "build"];

/// Joins after which the command's exit is no longer the check's.
const MASKS: &[&str] = &["|", ";", "\n"];

/// Whether `cmd`'s exit is a check's: no join but `&&`, and a segment whose
/// first word, past any `NAME=value` settings, is one of [`RUNNERS`] and
/// that names one of [`REASONS`].
pub fn is_check(cmd: &str) -> bool {
    if MASKS.iter().any(|m| cmd.contains(m)) {
        return false;
    }
    cmd.split("&&").any(|s| {
        let lower = s.to_ascii_lowercase();
        s.split_whitespace()
            .find(|w| !w.contains('='))
            .is_some_and(|first| RUNNERS.contains(&first))
            && REASONS.iter().any(|r| lower.contains(r))
    })
}

#[cfg(test)]
#[path = "checkrun_tests.rs"]
mod tests;
