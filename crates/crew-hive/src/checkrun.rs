//! Whether a shell command is a check: a build, a test or a lint run.
//!
//! WHY here: the broker already keeps the list of runners it will believe a
//! check starts with (`checkcmd`, for the command a repo declares), and the
//! advisor now needs the same answer for a command the agent ran — "did a
//! check pass after the last edit?" (`advisor::done`). One list, in the crate
//! both can reach, so the two can never disagree about what a check is.
//!
//! What a check is stays the list's claim, not a guess at intent: a runner
//! that only builds, tests or lints. `cargo run` or `npm install` start with
//! a runner too and count; the cost of that is one advisor read not asked
//! for, never one asked for wrongly.

/// Commands crew will believe are a check. Everything here builds, tests or
/// lints; nothing here moves files, talks to a network or installs anything
/// on its own.
pub const RUNNERS: &[&str] = &[
    "cargo", "npm", "pnpm", "yarn", "bun", "deno", "make", "just", "pytest", "tox", "go", "mvn",
    "gradle", "dotnet", "swift", "zig", "rake", "bundle", "ninja", "ctest", "mix", "stack",
];

/// Shell syntax that joins one command to the next. An agent's run is often
/// `cd crates/x && cargo test`: the check is a segment, not the first word.
const JOINS: &[&str] = &["&&", "||", ";", "|", "\n"];

/// Whether `cmd` runs a check in one of its segments: a segment whose first
/// word, past any `NAME=value` settings, is one of [`RUNNERS`].
pub fn is_check(cmd: &str) -> bool {
    let mut segments = vec![cmd];
    for join in JOINS {
        segments = segments.iter().flat_map(|s| s.split(join)).collect();
    }
    segments.iter().any(|s| {
        s.split_whitespace()
            .find(|w| !w.contains('='))
            .is_some_and(|first| RUNNERS.contains(&first))
    })
}

#[cfg(test)]
#[path = "checkrun_tests.rs"]
mod tests;
