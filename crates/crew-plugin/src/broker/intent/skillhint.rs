//! The router chooses the playbooks too: one call before the work, not three.
//!
//! Measured on a live DashScope run (2026-09-27): a plain question paid for
//! the routing call AND two skill calls before the first token — the context
//! line asked on the raw message, the relay asked again on the same message
//! wrapped in recalled turns (a different text, so a memo miss), and both
//! answered `SKILLS: none`. At ~1.1 s a call that was two seconds of dead air
//! on every message. The routing call already reads the message; it now
//! reads the skill roster beside it and may answer one more line,
//! `SKILLS: <name, name>`, which is seeded into the decider's memo so every
//! arm that frames the task finds the decision already made.
//!
//! A router that says nothing about skills (an older reply shape, a failed
//! or off-grammar call) seeds nothing, and the decider asks on its own
//! exactly as before — the fallback is the old path, untouched.
use crate::broker::route::clip;
use crate::broker::skillchoice::{self, AUTO_MAX};

/// Chars of a skill's one-liner on its roster row — the chooser's own cap.
const DESC_CAP: usize = 80;

/// `(name, one-liner)` for each loaded skill, or nothing when there is no
/// choice to make: fewer than two skills, or `CREW_SKILL_PICK=0`.
pub(crate) fn rows() -> Vec<(String, String)> {
    if skillchoice::disabled() {
        return Vec::new();
    }
    let skills = crate::broker::skills::load();
    if skills.len() < AUTO_MAX {
        return Vec::new();
    }
    skills
        .iter()
        .map(|s| {
            let one = s.description.lines().next().unwrap_or("");
            (
                s.name.clone(),
                clip(one.trim_start_matches('#').trim(), DESC_CAP),
            )
        })
        .collect()
}

/// The grammar sentence the routing prompt gains when there are skills.
///
/// Worded against what was measured (2026-09-28, qwen-flash): a one-line
/// doc comment drew `doc-coauthoring`, a palette tweak in a `crew-theme`
/// path drew `theme-factory`, and "with tests and docs" drew two more. The
/// earlier sentence already said a name appearing is no reason; the model
/// read a word ABOUT the task as the task. So the default is said first,
/// and the test is the description, not the name. The doc comment and the
/// tests-and-docs work stopped drawing skills; the palette tweak still draws
/// `theme-factory`, and `routereval` counts it a miss.
pub(crate) const GRAMMAR: &str = " One more optional line `SKILLS: <name, name>` \
     (reply, swarm, loop or goal) names up to two of the skills listed below. \
     Most messages need none, and the line is then `SKILLS: none`. Name a \
     skill only when the message asks for the kind of work its DESCRIPTION \
     describes \u{2014} never because a word in the message (a file, crate or \
     path name, or any word that looks like the skill's name) matches it.";

/// The roster block for the world section: one row per skill.
pub(crate) fn block(rows: &[(String, String)]) -> Option<String> {
    if rows.is_empty() {
        return None;
    }
    let lines: Vec<String> = rows
        .iter()
        .map(|(n, d)| format!("  {n} \u{2014} {d}"))
        .collect();
    Some(format!("skills:\n{}", lines.join("\n")))
}

/// The `SKILLS:` line from anywhere after the first, against `names` (exact,
/// case-insensitive, roster order, at most [`AUTO_MAX`]). `Some(empty)` is
/// the router's "none"; `None` is no decision — no line, or one naming
/// nothing known — and leaves the choice to the decider.
pub(crate) fn parse(reply: &str, names: &[String]) -> Option<Vec<String>> {
    let tail = reply.trim().lines().skip(1).find_map(|l| {
        let (head, tail) = l.trim().split_once(':')?;
        head.trim_matches(|c: char| matches!(c, '*' | '`' | '_'))
            .trim()
            .eq_ignore_ascii_case("skills")
            .then_some(tail.trim())
    })?;
    if tail.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    let said: Vec<&str> = tail
        .split([',', ' '])
        .map(|w| w.trim_matches(|c: char| matches!(c, '`' | '*' | '_' | '.' | ';' | '"')))
        .filter(|w| !w.is_empty())
        .collect();
    let picked: Vec<String> = names
        .iter()
        .filter(|n| said.iter().any(|w| w.eq_ignore_ascii_case(n)))
        .take(AUTO_MAX)
        .cloned()
        .collect();
    (!picked.is_empty()).then_some(picked)
}

/// Hand the router's choice for `task` to the decider, so the arms' own
/// picks on the same text are memo hits instead of calls.
pub(crate) fn seed(task: &str, names: &[String]) {
    let skills = crate::broker::skills::load();
    skillchoice::decider().seed(task, &skills, names);
}
