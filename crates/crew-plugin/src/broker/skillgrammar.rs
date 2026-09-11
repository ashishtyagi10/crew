//! The skill chooser's grammar: the prompt the model is shown and the strict
//! reader of its first line. Apart from `skillchoice` so the decider stays
//! inside the line cap; a child of it, so `broker` items are reached through
//! `crate::broker::`.
use super::{by_name, AUTO_MAX};
use crate::broker::route::clip;
use crate::broker::skills::Skill;

/// Chars of the task carried into the prompt.
const TASK_CAP: usize = 1_500;
/// Chars of a skill's one-liner on its roster row.
const DESC_CAP: usize = 80;

/// The prompt: the grammar first, the roster next, the task last.
pub(crate) fn prompt(task: &str, skills: &[Skill]) -> String {
    let rows: Vec<String> = skills
        .iter()
        .map(|s| format!("{} \u{2014} {}", s.name, clip(one_liner(s), DESC_CAP)))
        .collect();
    format!(
        "You choose which skills (drop-in playbooks) ONE agent follows for its task. The \
         FIRST line of your reply must be exactly `SKILLS: <name>, <name>` \u{2014} up to \
         {AUTO_MAX} names, spelled exactly as listed below, only the playbooks whose work \
         this task IS \u{2014} or `SKILLS: none` when none of them fits. A name appearing in \
         the task is not a reason by itself: the task has to be what the playbook is for. \
         Nothing else.\n\nSkills:\n{}\n\nTask: {}",
        rows.join("\n"),
        clip(task, TASK_CAP)
    )
}

/// The skill's one line for the roster: its description's first line (the
/// parser fills it from the body's first line when there is no frontmatter,
/// so a leading heading mark is shed).
fn one_liner(s: &Skill) -> &str {
    let desc = s.description.lines().next().unwrap_or("");
    desc.trim_start_matches('#').trim()
}

/// The names a `SKILLS:` line keeps, in the ROSTER's order: exact names only
/// (case-insensitive), unknown names dropped, duplicates collapsed, at most
/// [`AUTO_MAX`] (the first named win). `SKILLS: none` is an empty choice.
/// `None` is NO choice — no `SKILLS:` first line, or one naming nothing known.
pub(crate) fn parse(reply: &str, skills: &[Skill]) -> Option<Vec<String>> {
    let first = reply.lines().map(str::trim).find(|l| !l.is_empty())?;
    let (head, tail) = first
        .trim_matches(|c: char| matches!(c, '*' | '`' | '_'))
        .split_once(':')?;
    if !head.trim().eq_ignore_ascii_case("skills") {
        return None;
    }
    let tail = tail.trim();
    if tail.eq_ignore_ascii_case("none") {
        return Some(Vec::new());
    }
    let mut named: Vec<&str> = Vec::new();
    for raw in tail.split([',', ' ']) {
        let name = raw.trim_matches(|c: char| matches!(c, '`' | '*' | '_' | '.' | ';' | '"'));
        let known = skills.iter().find(|s| s.name.eq_ignore_ascii_case(name));
        if let Some(k) = known.filter(|k| !named.contains(&k.name.as_str())) {
            named.push(&k.name);
        }
        if named.len() == AUTO_MAX {
            break;
        }
    }
    if named.is_empty() {
        return None;
    }
    Some(
        by_name(&named, skills)
            .iter()
            .map(|s| s.name.clone())
            .collect(),
    )
}
