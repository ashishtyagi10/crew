//! The folded verbs: one command, a list of subjects, one row in the palette.
//!
//! `/look` (v0.22.25) was the first — fifteen appearance commands under one
//! verb — and the shape turned out to be the general one. A family of
//! commands that differ by a WORD (`/clear`, `/clearall`, `/clearlog`;
//! `/find`, `/findall`; `/errors`, `/errorsall`; `/closeall`, `/only`) is a
//! family where the word belongs in the argument, not in the name: it keeps
//! the palette short, it puts the variants where you can see them the moment
//! you type the verb, and it stops the fuzzy matcher having to choose
//! between two rows that mean almost the same thing.
//!
//! Every folded spelling still runs (`cmddefs::answered`) — see `lookcmd`'s
//! doc for why folding is not retiring.
use crate::lookcmd;

/// A verb and the subjects it owns.
pub(crate) struct Verb {
    pub name: &'static str,
    pub subjects: &'static [(&'static str, &'static str)],
    /// Whether a subject opens a picker of its OWN (`/look gamma ` → the
    /// gamma ladder). False when the subject is the whole answer.
    pub two_step: bool,
}

/// `/clear`'s subjects, and the rest. `/look`'s live in `lookcmd`, where the
/// forwarding to each knob's own command lives too.
pub(crate) const VERBS: &[Verb] = &[
    Verb {
        name: "/look",
        subjects: lookcmd::SUBJECTS,
        two_step: true,
    },
    Verb {
        name: "/clear",
        subjects: &[
            ("all", "every pane's scrollback, not just this one"),
            ("log", "the live activity log in the sidebar"),
        ],
        two_step: false,
    },
    Verb {
        name: "/close",
        subjects: &[
            ("all", "close every pane"),
            ("others", "close every pane but the focused one"),
        ],
        two_step: false,
    },
    Verb {
        name: "/errors",
        subjects: &[("all", "which panes have errors, and how many")],
        two_step: false,
    },
    Verb {
        name: "/nav",
        subjects: &[
            (
                "glance",
                "the glance cards: serving, waiting on you, weather",
            ),
            ("log", "the live activity log"),
            (
                "weather",
                "the clock's weather strip (/nav weather <place>)",
            ),
        ],
        two_step: false,
    },
    Verb {
        name: "/out",
        subjects: &[("list", "every command block, to open one by number")],
        two_step: false,
    },
    Verb {
        name: "/find",
        subjects: &[("all", "search every pane's scrollback (/find all <text>)")],
        two_step: false,
    },
];

fn verb(name: &str) -> Option<&'static Verb> {
    VERBS.iter().find(|v| v.name == name)
}

/// The rows for a verb itself — `None` for every other command, so the
/// caller falls through to the value pickers (`suggestvalues`).
pub(crate) fn options(cmd: &str) -> Option<Vec<(String, String)>> {
    verb(cmd).map(|v| {
        v.subjects
            .iter()
            .map(|(s, d)| (s.to_string(), d.to_string()))
            .collect()
    })
}

/// Read a typed command and argument as the picker needs them:
/// `(command to look values up under, the argument, the text a row fills)`.
/// Only `/look` redirects — its subjects ARE other commands; the rest own
/// their arguments outright.
pub(crate) fn canon(cmd: &str, arg: &str) -> (String, String, String) {
    lookcmd::canon(cmd, arg)
}

/// Whether a row is a subject that opens a picker of its own — those INSERT
/// (with the space the next step needs) instead of running.
pub(crate) fn nested(prefix: &str, value: &str) -> bool {
    verb(prefix).is_some_and(|v| v.two_step && v.subjects.iter().any(|(s, _)| *s == value))
}

/// What a row types into the bar.
pub(crate) fn fill(prefix: &str, value: &str) -> String {
    match nested(prefix, value) {
        true => format!("{prefix} {value} "),
        false => format!("{prefix} {value}"),
    }
}

/// The command whose CURRENT value the bar should mark for `text`.
pub(crate) fn current_key(text: &str) -> String {
    lookcmd::current_key(text)
}

/// The spellings that were folded into a verb and still answer, though no
/// palette row offers them (`cmddefs::answered`).
pub(crate) const FOLDED: &[&str] = &[
    "weather",
    "blocks",
    "md",
    "findall",
    "errorsall",
    "clearall",
    "clearlog",
    "closeall",
    "only",
];

pub(crate) fn is_folded(name: &str) -> bool {
    FOLDED.contains(&name) || lookcmd::is_subject(name)
}

#[cfg(test)]
#[path = "verbs_tests.rs"]
mod tests;
