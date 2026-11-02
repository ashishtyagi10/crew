//! What a shell command does to the proof: which parts of it build or test,
//! and which change files.
//!
//! The proof once split a command at every non-letter and looked for words
//! like "test", so `wc -l src/x_tests.rs` passed for a test run and `sed -i`
//! never counted as an edit. Now each piece of a command (split at `&&`,
//! `||`, `;`, `|` and line breaks) is judged by its program: a runner
//! verifies, a writer edits, anything else is neither. A command can do
//! both, in order: `cargo fmt && cargo test` edits, then checks.

/// What one piece of a command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    Verify,
    Write,
}

/// Each piece's effect, in the order the shell runs them.
pub(crate) fn effects(cmd: &str) -> Vec<Effect> {
    pieces(cmd).filter_map(|p| effect(&p)).collect()
}

fn pieces(cmd: &str) -> impl Iterator<Item = Vec<String>> + '_ {
    cmd.split(['\n', ';', '|', '&'])
        .map(|s| s.split_whitespace().map(unquote).collect::<Vec<_>>())
        .filter(|w| !w.is_empty())
}

fn unquote(w: &str) -> String {
    w.trim_matches(|c| c == '"' || c == '\'' || c == '(' || c == ')')
        .to_string()
}

/// Wrappers in front of the real program: `CI=1 time npx jest`.
const WRAPPERS: &[&str] = &["env", "time", "nice", "npx", "bunx", "sudo", "exec"];

fn effect(words: &[String]) -> Option<Effect> {
    let mut w = words.iter().map(String::as_str).skip_while(|w| {
        WRAPPERS.contains(w) || (w.contains('=') && !w.starts_with('-') && !w.starts_with('>'))
    });
    let prog = w.next()?;
    let rest: Vec<&str> = w.collect();
    if redirects_into_tree(&rest) {
        return Some(Effect::Write);
    }
    let name = prog.rsplit('/').next().unwrap_or(prog);
    let (sub, has) = (rest.first().copied().unwrap_or(""), |f: &str| {
        rest.contains(&f)
    });
    let runner = match name {
        "cargo" => match sub {
            "test" | "nextest" | "check" | "build" | "clippy" => true,
            "fmt" if !has("--check") => return Some(Effect::Write),
            "fmt" => true,
            _ => false,
        },
        "npm" | "pnpm" | "yarn" | "bun" => {
            let sub = if sub == "run" {
                rest.get(1).copied().unwrap_or("")
            } else {
                sub
            };
            ["test", "build", "lint", "typecheck", "check", "tsc", "t"]
                .iter()
                .any(|k| sub == *k || sub.starts_with(&format!("{k}:")))
        }
        "go" => matches!(sub, "test" | "build" | "vet"),
        "python" | "python3" => sub == "-m" && rest.get(1).is_some_and(|m| PY.contains(m)),
        "swift" | "dotnet" => matches!(sub, "test" | "build"),
        "git" => {
            let edits = [
                "apply",
                "checkout",
                "restore",
                "am",
                "cherry-pick",
                "merge",
                "rebase",
            ];
            return (edits.contains(&sub) || (sub == "reset" && has("--hard")))
                .then_some(Effect::Write);
        }
        "sed" | "perl" => {
            return rest
                .iter()
                .any(|f| f.starts_with("-i") || f.starts_with("-pi"))
                .then_some(Effect::Write)
        }
        "rustfmt" | "gofmt" | "patch" | "mv" | "cp" | "rm" | "touch" => {
            return (name != "gofmt" || has("-w")).then_some(Effect::Write)
        }
        "tee" => return rest.iter().any(|f| in_tree(f)).then_some(Effect::Write),
        "prettier" | "black" => {
            return (has("--write") || name == "black" && !has("--check")).then_some(Effect::Write)
        }
        _ => RUNNERS.contains(&name) || name.contains("test") || name.starts_with("check"),
    };
    runner.then_some(Effect::Verify)
}

const PY: &[&str] = &["pytest", "mypy", "unittest", "tox", "ruff"];

const RUNNERS: &[&str] = &[
    "pytest",
    "make",
    "tsc",
    "mypy",
    "ruff",
    "eslint",
    "jest",
    "vitest",
    "ctest",
    "rspec",
    "gradle",
    "gradlew",
    "mvn",
    "mvnw",
    "tox",
    "nox",
    "xcodebuild",
    "nextest",
];

/// `> file` or `>> file` into somewhere other than a scratch place.
fn redirects_into_tree(rest: &[&str]) -> bool {
    rest.iter().enumerate().any(|(i, w)| {
        let target = match *w {
            ">" | ">>" => rest.get(i + 1).copied(),
            w if w.starts_with(">>") => Some(&w[2..]),
            w if w.starts_with('>') => Some(&w[1..]),
            _ => None,
        };
        target.is_some_and(|t| !t.is_empty() && !t.starts_with('&') && in_tree(t))
    })
}

/// Not a scratch path: `/dev/null`, `/tmp/…` and `$TMPDIR` writes are logs.
fn in_tree(path: &str) -> bool {
    let scratch = [
        "/dev/",
        "/tmp/",
        "/private/tmp/",
        "/var/folders/",
        "$TMPDIR",
        "${TMPDIR",
    ];
    !path.starts_with('-') && !scratch.iter().any(|s| path.starts_with(s))
}

#[cfg(test)]
#[path = "chatproofcmd_tests.rs"]
mod tests;
