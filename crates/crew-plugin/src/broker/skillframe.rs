//! Skill framing: how a playbook is presented to the relay. Small bodies are
//! inlined whole (as `/skill` always did); directory skills add a pointer to
//! their bundled files, readable through the `@tool sys` surface. WHICH
//! playbooks a task gets is `skillchoice`'s decision (the model's, with the
//! name match as its fallback); this file only frames what was chosen.
use super::skills::Skill;
use crate::PluginEvent;

/// Bodies over this many bytes are pointer-framed instead of inlined (~2k tokens).
pub(crate) const INLINE_CAP: usize = 8 * 1024;

/// One playbook a task pulled in — what the host is told about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Applied {
    pub name: String,
    /// The skill's one-liner (frontmatter, or its first line).
    pub description: String,
    /// The model chose it (`chose`), or the task named it (`applied`).
    pub chosen: bool,
}

/// A task with its skills woven in, and WHICH skills — the frame used to be
/// the only output, so a playbook rewrote the prompt and nothing anywhere
/// said so; the pane draws one line per entry of `applied`.
pub(crate) struct Framed {
    pub body: String,
    pub applied: Vec<Applied>,
}

/// Weave the loaded skills into `task` without any command: the playbooks
/// the decider picked are framed in full ([`framed`]); when none are but
/// skills exist, a one-line roster rides along so the model knows what it
/// could name. No skills → the task passes through byte-identical.
pub(crate) fn with_skills(task: &str) -> Framed {
    let skills = super::skills::load();
    let pick = super::skillchoice::decider().pick(task, &skills);
    let applied = pick
        .skills
        .iter()
        .map(|s| Applied {
            name: s.name.clone(),
            description: s.description.clone(),
            chosen: pick.by_model,
        })
        .collect();
    Framed {
        body: frame_with(task, &skills, &pick.skills, super::systools::enabled()),
        applied,
    }
}

/// The `Loaded` events announcing `applied` to the host, one per skill,
/// attributed to `agent` — the agent whose prompt the playbook now heads.
pub(crate) fn loaded_events(applied: &[Applied], agent: &str) -> Vec<PluginEvent> {
    applied
        .iter()
        .map(|a| PluginEvent::Hive {
            event: crew_hive::HiveEvent::Loaded {
                agent: agent.to_string(),
                kind: "skill".into(),
                name: a.name.clone(),
                detail: format!(
                    "{} \u{b7} {}",
                    if a.chosen { "chose" } else { "applied" },
                    a.description
                ),
            },
        })
        .collect()
}

/// [`frame_with`] on the name match alone — the keyless/mock body; the tests' seam.
#[cfg(test)]
pub(crate) fn auto_frame(task: &str, skills: &[Skill], sys_on: bool) -> String {
    let named = super::skillchoice::matched(task, skills);
    frame_with(task, skills, &named, sys_on)
}

/// Pure core of [`with_skills`]'s body: `task` under the `chosen` playbooks.
fn frame_with(task: &str, skills: &[Skill], chosen: &[&Skill], sys_on: bool) -> String {
    if skills.is_empty() {
        return task.to_string();
    }
    if chosen.is_empty() {
        return format!(
            "AVAILABLE SKILLS (drop-in playbooks \u{2014} none chosen for this \
             task):\n{}\n\nTASK:\n{task}",
            list_report(skills)
        );
    }
    if let [only] = chosen {
        return framed(only, task, sys_on);
    }
    let blocks: Vec<String> = chosen.iter().map(|s| block(s, sys_on)).collect();
    format!("{}\nTASK:\n{task}", blocks.join("\n"))
}

/// The relay body for a skill run: playbook first, then the task. Oversized
/// playbooks become a pointer frame — description, intro, heading outline, and
/// the path to read on demand — when the agent has `sys` tools to follow it;
/// without them a pointer would be a dead end, so everything inlines.
pub(crate) fn framed(skill: &Skill, task: &str, sys_on: bool) -> String {
    format!("{}\nTASK:\n{task}", block(skill, sys_on))
}

/// The playbook block alone — everything [`framed`] puts above the task.
fn block(skill: &Skill, sys_on: bool) -> String {
    if !sys_on || skill.body.len() <= INLINE_CAP {
        return format!(
            "SKILL \u{201c}{}\u{201d} \u{2014} follow this playbook:\n{}\n{}",
            skill.name,
            skill.body,
            support(skill)
        );
    }
    let heads: Vec<&str> = skill
        .body
        .lines()
        .filter(|l| l.starts_with("## ") || l.starts_with("### "))
        .collect();
    let outline = if heads.is_empty() {
        String::new()
    } else {
        format!("Outline:\n{}\n", heads.join("\n"))
    };
    format!(
        "SKILL \u{201c}{}\u{201d} \u{2014} {}\n{}\n{outline}Full playbook: {} \u{2014} \
         read the sections you need with @tool sys:read_file \
         {{\"path\": \u{2026}, \"offset\": \u{2026}}} before starting.\n{}",
        skill.name,
        skill.description,
        intro(&skill.body),
        skill.path.display(),
        support(skill)
    )
}

/// Body text before the first `##` heading, byte-clipped at 1 KB on a char
/// boundary — the playbook's own preamble, kept as scene-setting.
fn intro(body: &str) -> &str {
    let head = if body.starts_with("## ") {
        ""
    } else {
        body.find("\n## ").map_or(body, |i| &body[..i])
    };
    if head.len() <= 1024 {
        return head.trim_end();
    }
    let mut cut = 1024;
    while !head.is_char_boundary(cut) {
        cut -= 1;
    }
    head[..cut].trim_end()
}

/// One line pointing a directory skill's agent at its bundled files.
fn support(skill: &Skill) -> String {
    match &skill.dir {
        Some(root) => format!(
            "\nSupporting files: {} \u{2014} read with @tool sys:read_file \
             {{\"path\": \u{2026}}}; run scripts with @tool sys:run.\n",
            root.display()
        ),
        None => String::new(),
    }
}

/// The skills roster: one line per skill, or where to put files.
pub(crate) fn list_report(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return "No skills found. Drop markdown playbooks into \
                ~/.config/crew/skills/ or ./.crew/skills/ \
                (optional `---` frontmatter: name, description); a task \
                that names one applies it by itself."
            .into();
    }
    let lines: Vec<String> = skills
        .iter()
        .map(|s| {
            let mut tag = s.origin.to_string();
            if s.dir.is_some() {
                tag.push_str(", dir");
            }
            if s.body.len() > INLINE_CAP {
                tag.push_str(&format!(", {} KB \u{2192} outline", s.body.len() / 1024));
            }
            format!("\u{25aa} {} \u{2014} {} ({tag})", s.name, s.description)
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
#[path = "skillframe_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "skillauto_tests.rs"]
mod auto_tests;
