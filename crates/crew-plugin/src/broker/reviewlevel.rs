//! A review comes in three levels, read from the asker's own words.
//!
//! Every review used to be one size: the diff, its callers, effort `high`.
//! "Give it a quick look" before a coffee break paid for a staff review, and
//! "a careful security review of auth" got no more than "review my diff".
//! Claude Code's review takes a level, and so does this one — read like the
//! review gate (`reviewask`), from a few plain words, never a model call:
//!
//! - **quick** ("quick look", "quickly", "glance", "skim"): the diff alone,
//!   no callers, effort `low`, the [`QUICK_MAX`] worst findings;
//! - **standard** (anything else): the review as it was;
//! - **deep** ("deep", "thorough", "careful", "security review"): callers in
//!   front, and each finding put back to the reviewer for a second look
//!   (`reviewverify`) before it is shown.
//!
//! Quick and deep say so on the reply's first line ("quick look · 3
//! findings"), so the person knows which review they got.
use crew_hive::Effort;

use super::reviewfindings::{finding, tidy, Cited, Tidied, SEVERITIES};

/// The most findings a quick look reports.
pub(crate) const QUICK_MAX: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    Quick,
    Standard,
    Deep,
}

/// Words asking for a light pass. Not "fast", "rough" or "brief": those
/// name the code as often as the look ("the diff that speeds up the fast
/// path"), and a quick look by mistake hides all but five findings.
const QUICK: &[&str] = &["quick look", "quickly", "glance", "skim"];

/// Words asking for a harder one. Said together with a quick word ("a quick
/// but thorough review"), deep wins: the cost of reading too much is time,
/// of reading too little a missed bug.
const DEEP: &[&str] = &[
    "deep",
    "deeply",
    "thorough",
    "thoroughly",
    "careful",
    "carefully",
    // As a phrase: "review my security fix" is a standard review.
    "security review",
    "security audit",
    "in-depth",
    "exhaustive",
    "line by line",
];

impl Level {
    /// The level `task` asks for.
    pub(crate) fn of_ask(task: &str) -> Level {
        let said = words(task);
        let has = |list: &[&str]| {
            list.iter().any(|p| {
                let want: Vec<&str> = p.split(' ').collect();
                said.windows(want.len()).any(|w| w == want.as_slice())
            })
        };
        if has(DEEP) {
            Level::Deep
        } else if has(QUICK) {
            Level::Quick
        } else {
            Level::Standard
        }
    }

    /// The effort this level's call thinks at, over the route's `high`;
    /// `None` leaves the route's.
    pub(crate) fn effort(self) -> Option<Effort> {
        match self {
            Level::Quick => Some(Effort::Low),
            Level::Standard => None,
            Level::Deep => Some(Effort::High),
        }
    }

    /// Whether the callers block rides in front of the diff.
    pub(crate) fn reads_callers(self) -> bool {
        self != Level::Quick
    }

    /// The first-line label, `None` for standard (whose reply is as it was).
    pub(crate) fn label(self, findings: usize) -> Option<String> {
        let name = match self {
            Level::Quick => "quick look",
            Level::Standard => return None,
            Level::Deep => "deep review",
        };
        Some(match findings {
            0 => format!("{name} · no findings"),
            1 => format!("{name} · 1 finding"),
            n => format!("{name} · {n} findings"),
        })
    }
}

/// The lowercase words of `task` a person said: code in backticks and any
/// token that reads as code or a path (`quick.rs`, `deep_merge`,
/// `fast::path`, `skim()`) or a hyphenated compound ("deep-link") is dropped
/// whole, so naming a thing never picks the level. "in-depth" is the one
/// compound that is the word itself.
fn words(task: &str) -> Vec<String> {
    let prose: String = task.split('`').step_by(2).collect::<Vec<_>>().join(" ");
    prose
        .split_whitespace()
        .filter(|t| !["()", "::", "_", "/", "\\"].iter().any(|c| t.contains(c)))
        .map(|t| {
            t.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|t| t == "in-depth" || !t.is_empty() && t.chars().all(char::is_alphanumeric))
        .collect()
}

/// The prompt for `level`: today's for standard, and the same contract with
/// the level's ask added for the others.
pub(crate) fn prompt(level: Level, diff: &str, callers: &str) -> String {
    let mut p = super::review::review_prompt(diff, callers);
    match level {
        Level::Standard => {}
        Level::Quick => p.push_str(&format!(
            "\n\nThis is a QUICK LOOK: report at most {QUICK_MAX} findings, \
             the worst first, and skip nits unless there is room."
        )),
        Level::Deep => p.push_str(
            "\n\nThis is a DEEP REVIEW: read every hunk and every caller \
             before judging; security, concurrency and error paths count.",
        ),
    }
    p
}

/// `t` with only its `max` most severe findings (blocker, then warn, then
/// nit; ties as written), the verdict recounted, and one line saying how
/// many were cut: the quick look's bound kept even when the model overran
/// it, without a blocker written seventh being the one to go. Runs after
/// the citation check, so a false finding never takes a true one's place.
pub(crate) fn worst(t: Tidied, max: usize) -> Tidied {
    let lines: Vec<&str> = t.text.lines().collect();
    let mut found: Vec<(usize, usize)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let (sev, _, _) = finding(l)?;
            Some((SEVERITIES.iter().position(|s| *s == sev)?, i))
        })
        .collect();
    if found.len() <= max {
        return t;
    }

    found.sort_by_key(|&(rank, _)| rank);
    let cut: Vec<usize> = found[max..].iter().map(|&(_, i)| i).collect();
    let text: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !cut.contains(i))
        .map(|(_, l)| *l)
        .collect();
    // Every citation was checked already; this only recounts the verdict.
    let mut shown = tidy(&text.join("\n"), &|_| Cited::Unknown);
    shown.text = format!(
        "{}\n{} more not shown — ask for a review for all",
        shown.text,
        cut.len()
    );
    shown.dropped = t.dropped;
    shown
}

/// The finding bullets in `text`, as the pane will draw them.
pub(crate) fn count(text: &str) -> usize {
    text.lines().filter(|l| finding(l).is_some()).count()
}

#[cfg(test)]
#[path = "reviewlevel_tests.rs"]
mod tests;
