//! The router may not send a message into git on its own say-so.
//!
//! Measured live (2026-09-28, qwen-flash on DashScope): "append the line
//! '// probe marker' at the very end of crates/crew-theme/src/lib.rs,
//! nothing else" came back `SHAPE: commit — Simple file modification
//! needed`, and the turn ended on "nothing to commit — the tree is clean".
//! The four shapes that act on git history or the previous session —
//! `commit`, `review`, `standup`, `resume` — each do ONE fixed thing and
//! never read the message, so a wrong pick answers a question nobody asked
//! and drops the one that was. Each is also asked for in a small, plain
//! vocabulary, so the check is a word list rather than another model call:
//! when the router picks one of them and the message says none of its
//! words, the message goes to one agent's reply, and the routing line says
//! the router was overruled rather than pretending it chose.
//!
//! Only the ROUTER's pick is checked. The gates ahead of it (`gate`: the
//! confirm that applies a drafted commit, the plan verdict) are exact
//! matches of their own, and plan-first never asks the router at all.
use super::decision::Decision;
use super::Shape;

/// The words that ask for each shape. Matched whole-word and
/// case-insensitive, a phrase as consecutive words, so `pr` is not found in
/// "sprint", `ship` not in "relationship", and `stand-up` also matches
/// "stand up". Generous on purpose: a word here only lets the router's pick
/// stand, it never makes one.
const ASKS: &[(Shape, &[&str])] = &[
    (Shape::Commit, &["commit"]),
    (
        Shape::Review,
        &[
            "review",
            "look over",
            "check my changes",
            "my diff",
            "the diff",
            "pr",
        ],
    ),
    (
        Shape::Standup,
        &[
            "standup",
            "stand-up",
            "ship",
            "shipped",
            "this week",
            "yesterday",
            "what did i",
        ],
    ),
    (
        Shape::Resume,
        &[
            "resume",
            "pick up",
            "left off",
            "last session",
            "where we were",
        ],
    ),
];

/// `d` as the router gave it, unless it picked a git or session shape that
/// `task` never asks for: then a reply, whose reason names what the router
/// said (and why, when it said why) so the line stays honest.
pub(super) fn hold(task: &str, d: Decision) -> Decision {
    let Some((_, words)) = ASKS.iter().find(|(shape, _)| *shape == d.shape) else {
        return d;
    };
    if says_any(task, words) {
        return d;
    }
    let said = d.shape.name();
    let why = match &d.why {
        Some(why) => format!("the router said {said} ({why}), but nothing here asks for one"),
        None => format!("the router said {said}, but nothing here asks for one"),
    };
    Decision {
        shape: Shape::Reply,
        why: Some(why),
        hints: d.hints,
    }
}

/// Whether `task` says any of `phrases`, as whole words. Paths and file
/// names are taken out first: "fix the bug in review.rs" names a file, and a
/// fix to it is not a code review.
fn says_any(task: &str, phrases: &[&str]) -> bool {
    let said = words(task, true);
    phrases.iter().any(|phrase| {
        let want = words(phrase, false);
        said.windows(want.len()).any(|w| w == want.as_slice())
    })
}

/// The lowercase words of `text`, split at anything that is not a letter or
/// a digit; with `skip_paths`, a whitespace token that looks like a path or
/// a file name is dropped whole before it is split.
fn words(text: &str, skip_paths: bool) -> Vec<String> {
    text.split_whitespace()
        .filter(|token| !(skip_paths && path_like(token)))
        .flat_map(|token| token.split(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// A token with a path separator, or a name with an extension (`review.rs`,
/// `README.md`), once quotes and sentence punctuation around it are shed —
/// so "commit." at the end of a sentence is still the word.
fn path_like(token: &str) -> bool {
    let t = token.trim_matches(|c: char| !c.is_alphanumeric() && !matches!(c, '/' | '\\' | '_'));
    if t.contains(['/', '\\']) {
        return true;
    }
    t.rsplit_once('.').is_some_and(|(stem, ext)| {
        !stem.is_empty() && !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

#[cfg(test)]
#[path = "shapeguard_tests.rs"]
mod tests;
