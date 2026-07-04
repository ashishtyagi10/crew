//! Skill framing: how a playbook is presented to the relay. Small bodies are
//! inlined whole (as `/skill` always did); directory skills add a pointer to
//! their bundled files, readable through the `@tool sys` surface.
use super::skills::Skill;

/// The relay body for a skill run: playbook first, then the task.
pub(crate) fn framed(skill: &Skill, task: &str, _sys_on: bool) -> String {
    inline(skill, task)
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

#[cfg(test)]
#[path = "skillframe_tests.rs"]
mod tests;
