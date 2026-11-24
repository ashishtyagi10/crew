//! Whether a shell command is a check: a build, a test or a lint run.
//!
//! WHY here: the broker keeps the list of runners it will believe a declared
//! check starts with (`checkcmd`, for the command a repo writes down), and the
//! advisor needs to know whether a command the agent RAN was a check — "did a
//! check pass after the last edit?" (`advisor::done`). [`RUNNERS`] and
//! [`REASONS`] are that strict list, shared so both start from the same
//! runners; `checkcmd` reads them unchanged.
//!
//! [`is_check`] is looser than `checkcmd`, because what it costs is
//! different: counting a check here only turns one advisor read off, while
//! `checkcmd` decides what crew runs unasked. Live, the read fired on turns
//! that had run `cargo test 2>&1 | tail -20`, `npx eslint` or
//! `python -m pytest`, which is to say on almost every turn. So it reads the
//! forms agents type ([`TOOLS`], [`VERBS`], [`LAUNCHERS`]), allows trailing
//! output filters ([`FILTERS`]), and matches whole words in subcommand place,
//! so `go get …/testify` or `cargo run --bin build_x` is no check.
//!
//! The trade, made knowingly: a filtered run exits with the filter's status,
//! so a failing `cargo test | tail` exits 0 and reads as a pass. `;`, `||`
//! and a newline still hand the exit to a later command and count as none.

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

/// Tools whose whole job is a check, so their name alone says so.
const TOOLS: &[&str] = &[
    "pytest", "tox", "ctest", "ruff", "eslint", "mypy", "tsc", "rspec", "jest", "vitest",
];

/// Subcommands that make a runner's run a check, past [`REASONS`]. A
/// `test:unit` script counts by its part before the colon.
const VERBS: &[&str] = &["clippy", "vet", "nextest", "typecheck", "compile"];

/// Runners only this trigger reads (`checkcmd` stays on [`RUNNERS`]).
const MORE_RUNNERS: &[&str] = &["./gradlew", "gradlew", "./mvnw"];

/// A subcommand that counts only with the flag beside it: `cargo fmt`
/// rewrites files, `cargo fmt --check` checks them.
const FLAGGED: &[(&str, &str)] = &[("fmt", "--check")];

/// Prefixes that run the command after them: `npx eslint`, `uv run pytest`.
const LAUNCHERS: &[&[&str]] = &[
    &["npx"],
    &["python", "-m"],
    &["python3", "-m"],
    &["uv", "run"],
    &["bundle", "exec"],
    &["poetry", "run"],
];

/// Runners whose `run` names a script (`npm run lint`), skipped to reach it.
const SCRIPTS: &[&str] = &["npm", "pnpm", "yarn", "bun"];

/// Pipe stages that only cut the output down.
const FILTERS: &[&str] = &["tail", "head", "grep"];

/// Joins after which the command's exit is a later command's.
const MASKS: &[&str] = &["||", ";", "\n"];

/// Whether `cmd` ran a check: no join but `&&` and trailing [`FILTERS`],
/// and a segment that, past `NAME=value` settings and a launcher, is one of
/// [`TOOLS`] or a runner whose subcommand is one of [`REASONS`] or [`VERBS`].
pub fn is_check(cmd: &str) -> bool {
    if MASKS.iter().any(|m| cmd.contains(m)) {
        return false;
    }
    let mut stages = cmd.split('|');
    let head = stages.next().unwrap_or_default();
    if !stages.all(|s| {
        s.split_whitespace()
            .next()
            .is_some_and(|w| FILTERS.contains(&w))
    }) {
        return false;
    }
    head.split("&&").any(|s| {
        let lower = s.to_ascii_lowercase();
        segment(&lower.split_whitespace().collect::<Vec<_>>())
    })
}

/// One `&&` segment's words.
fn segment(words: &[&str]) -> bool {
    let mut w = words
        .iter()
        .position(|w| !w.contains('='))
        .map_or(&[][..], |i| &words[i..]);
    if let Some(l) = LAUNCHERS.iter().find(|l| w.starts_with(l)) {
        w = &w[l.len()..];
    }
    let Some((&first, rest)) = w.split_first() else {
        return false;
    };
    if TOOLS.contains(&first) {
        return true;
    }
    if !RUNNERS.contains(&first) && !MORE_RUNNERS.contains(&first) {
        return false;
    }
    let mut subs = rest.iter().filter(|w| !w.starts_with(['-', '+']));
    let mut sub = subs.next();
    if SCRIPTS.contains(&first) && sub == Some(&"run") {
        sub = subs.next();
    }
    let Some(&sub) = sub else { return false };
    let word = sub.split(':').next().unwrap_or(sub);
    REASONS.contains(&word)
        || VERBS.contains(&word)
        || TOOLS.contains(&word)
        || FLAGGED.iter().any(|(s, f)| *s == word && rest.contains(f))
}

#[cfg(test)]
#[path = "checkrun_tests.rs"]
mod tests;
