//! The model chooses which skills a task pulls in; the name match is the fallback.
//!
//! WHY: a playbook applied itself when the task CONTAINED its name — "review my
//! deploy plan" pulled in `deploy`, and a task that was plainly a code review
//! without the word got nothing. A substring is not a judgement. agent smith is
//! the brain of the run; which playbook a task follows is a decision, and
//! decisions are the model's. The name match never co-decides: it answers only
//! when the model could not (off, keyless, mock, a failed call, an off-grammar
//! reply), exactly as it always did.
//!
//! The shape is `toolchoice`'s: one bounded call, a strict first-line grammar
//! (`skillgrammar`), an unparsable reply that falls back rather than guesses,
//! and a per-task memo (`toolmemo::Memo`) so one decision is one call. No call
//! with fewer than two skills: there is nothing to choose between.
//!
//! `CREW_SKILL_PICK=0` turns the model step off (name match only).
use std::sync::OnceLock;

use super::session::toolchoice::ChooseFn;
use super::session::toolmemo::Memo;
use super::skills::Skill;

/// How many playbooks one task may pull in — a bound, so a task that happens
/// to name half the library cannot flood its own prompt.
pub(crate) const AUTO_MAX: usize = 2;
/// Output ceiling: `SKILLS:` and two names sit well under it.
const CHOICE_MAX_TOKENS: u32 = 128;
/// Decisions remembered: a run's width and a retry or two, not a history.
const MEMO_CAP: usize = 16;

/// `CREW_SKILL_PICK=0` — the only env read in this file. Name match only, no call, ever.
pub(crate) fn disabled() -> bool {
    std::env::var("CREW_SKILL_PICK").is_ok_and(|v| v == "0")
}

/// The live call, when one may run: routing's gates (`CREW_INTENT=0`, keyless,
/// mock) and this file's own. Skills are chosen before a run's runtime exists,
/// so unlike `toolchoice` the call needs no thread of its own.
pub(crate) fn live() -> Option<Box<ChooseFn>> {
    if disabled() {
        return None;
    }
    let call = super::intent::live_call(CHOICE_MAX_TOKENS)?;
    Some(Box::new(call))
}

/// What a task pulled in, and WHO decided — the pane says which.
pub(crate) struct Pick<'s> {
    pub skills: Vec<&'s Skill>,
    /// The model chose (`true`) or the name match answered (`false`).
    pub by_model: bool,
}

/// Who decides. `Live` resolves the provider at the moment of need; the other
/// two are the test seams, and the switch is read at pick time.
#[cfg_attr(not(test), allow(dead_code))]
enum Mode {
    Live,
    Off,
    Fixed(Box<ChooseFn>),
}

/// The skill decider: the chooser and its per-task memo. One for the process
/// (see [`decider`]) — skills are loaded per task, not per session, and the
/// memo's key carries the roster, so a library that changes is a new question.
pub(crate) struct Decider {
    mode: Mode,
    memo: Memo<(Vec<String>, bool)>,
}

/// The process's decider, built on first use.
pub(crate) fn decider() -> &'static Decider {
    static LIVE: OnceLock<Decider> = OnceLock::new();
    LIVE.get_or_init(Decider::live)
}

impl Decider {
    pub(crate) fn live() -> Self {
        Self::with(Mode::Live)
    }
    /// Name match only — the test surfaces, where a real call would be a network call.
    #[cfg(test)]
    pub(crate) fn off() -> Self {
        Self::with(Mode::Off)
    }
    /// An injected chooser — the seam the tests decide through.
    #[cfg(test)]
    pub(crate) fn fixed(f: Box<ChooseFn>) -> Self {
        Self::with(Mode::Fixed(f))
    }
    fn with(mode: Mode) -> Self {
        Self {
            mode,
            memo: Memo::new(MEMO_CAP),
        }
    }

    /// The skills `task` pulls in: the memo's answer, else the model's, else
    /// the name match's. Under two skills, or with the step off, no call.
    pub(crate) fn pick<'s>(&self, task: &str, skills: &'s [Skill]) -> Pick<'s> {
        if skills.len() < AUTO_MAX || disabled() {
            return fallback(task, skills);
        }
        let key = memo_key(task, skills);
        if let Some((names, by_model)) = self.memo.get(&key) {
            return Pick {
                skills: by_name(&names, skills),
                by_model,
            };
        }
        let held;
        let chooser: Option<&ChooseFn> = match &self.mode {
            Mode::Off => None,
            Mode::Fixed(f) => Some(&**f),
            Mode::Live => {
                held = live();
                held.as_deref()
            }
        };
        // No chooser is no decision: nothing was asked, nothing is remembered.
        let Some(call) = chooser else {
            return fallback(task, skills);
        };
        let pick = match choose(task, skills, call) {
            Some(names) => Pick {
                skills: by_name(&names, skills),
                by_model: true,
            },
            None => fallback(task, skills),
        };
        let names = pick.skills.iter().map(|s| s.name.clone()).collect();
        self.memo.put(key, (names, pick.by_model));
        pick
    }
}

/// The model's choice for `task`, or `None` when the name match must decide:
/// a failed call or an off-grammar reply.
pub(crate) fn choose(task: &str, skills: &[Skill], call: &ChooseFn) -> Option<Vec<String>> {
    parse(&call(&prompt(task, skills)).ok()?, skills)
}

/// The roster's skills among `names`, in roster order.
fn by_name<'s, S: AsRef<str>>(names: &[S], skills: &'s [Skill]) -> Vec<&'s Skill> {
    skills
        .iter()
        .filter(|s| names.iter().any(|n| n.as_ref() == s.name))
        .collect()
}

/// Today's answer, and the one every stop lands on: the playbooks `task`
/// names, in library order, at most [`AUTO_MAX`].
fn fallback<'s>(task: &str, skills: &'s [Skill]) -> Pick<'s> {
    Pick {
        skills: matched(task, skills),
        by_model: false,
    }
}

/// The playbooks `task` names, in library order, at most [`AUTO_MAX`].
pub(crate) fn matched<'s>(task: &str, skills: &'s [Skill]) -> Vec<&'s Skill> {
    skills
        .iter()
        .filter(|s| mentioned(task, &s.name))
        .take(AUTO_MAX)
        .collect()
}

/// Whether `task` names the skill: the normalized name, with hyphens also
/// matching spaces ("code-review" ↔ "code review"), case-insensitively.
fn mentioned(task: &str, name: &str) -> bool {
    let low = task.to_lowercase();
    low.contains(name) || low.contains(&name.replace('-', " "))
}

/// The memo key: the task AND the roster's spelling, so a skill dropped into
/// the library mid-session makes the same task a new question.
fn memo_key(task: &str, skills: &[Skill]) -> String {
    let mut k = skills
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(",");
    k.push('\n');
    k.push_str(task);
    k
}

#[path = "skillgrammar.rs"]
mod grammar;
pub(crate) use grammar::{parse, prompt};

#[cfg(test)]
#[path = "skillchoice_tests.rs"]
mod tests;
