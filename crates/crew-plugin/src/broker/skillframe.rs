//! Skill framing: how a playbook is presented to the relay. Small bodies are
//! inlined whole (as `/skill` always did); directory skills add a pointer to
//! their bundled files, readable through the `@tool sys` surface.
use super::skills::Skill;

/// Bodies over this many bytes are pointer-framed instead of inlined (~2k tokens).
pub(crate) const INLINE_CAP: usize = 8 * 1024;

/// The relay body for a skill run: playbook first, then the task. Oversized
/// playbooks become a pointer frame — description, intro, heading outline, and
/// the path to read on demand — when the agent has `sys` tools to follow it;
/// without them a pointer would be a dead end, so everything inlines.
pub(crate) fn framed(skill: &Skill, task: &str, sys_on: bool) -> String {
    if !sys_on || skill.body.len() <= INLINE_CAP {
        return inline(skill, task);
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
         {{\"path\": \u{2026}, \"offset\": \u{2026}}} before starting.\n{}\nTASK:\n{task}",
        skill.name,
        skill.description,
        intro(&skill.body),
        skill.path.display(),
        support(skill)
    )
}

/// The full-body frame (identical to the historical format for flat skills).
fn inline(skill: &Skill, task: &str) -> String {
    format!(
        "SKILL \u{201c}{}\u{201d} \u{2014} follow this playbook:\n{}\n{}\nTASK:\n{task}",
        skill.name,
        skill.body,
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

/// The `/skills` listing: one line per skill, or where to put files.
pub(crate) fn list_report(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return "No skills found. Drop markdown playbooks into \
                ~/.config/crew/skills/ or ./.crew/skills/ \
                (optional `---` frontmatter: name, description), then \
                run one with /skill <name> <task>."
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
