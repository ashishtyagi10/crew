//! Skills for an agent that opens files itself: the catalog, not a pick.
//!
//! Before each relay turn crew asked a model which playbooks the task needs
//! (`skillchoice`). Signed in to Claude Code, that model IS Claude Code — a
//! second `claude -p` started before the turn's own — and every plain
//! message waited on it: 4.6–7.0 s wall on 2026-10-30 (three calls, 17
//! skills installed), most of it the CLI starting, before the agent began.
//! An agent with file tools of its own needs no one to choose for it.
//! Claude Code's own skills work this way — each skill's name and one line
//! in front of the model, the body read when the task calls for it — so
//! that is what it is handed: every skill's name, its line and the file to
//! read, and the decision is its own, inside the turn it was going to run
//! anyway.
use super::skillframe::Framed;
use super::skills::Skill;
use crate::Broker;

/// Chars of a skill's one line the catalog keeps.
const LINE_CAP: usize = 100;

/// `task` framed for `agent`: the catalog when it reads files itself, else
/// the playbooks the decider picks on `request` (`skillframe`).
pub(crate) fn framed(broker: &Broker, agent: &str, request: &str, task: &str) -> Framed {
    match broker.registry.get(agent).is_some_and(|a| a.reads_files()) {
        true => catalog(task, &super::skills::load()),
        false => super::skillframe::with_skills_on(request, task),
    }
}

/// `task` under the list of `skills`, each with the file that holds it. No
/// skills: the task, byte for byte. Nothing is announced as applied — the
/// agent's own Read of a playbook is the line the pane shows.
pub(crate) fn catalog(task: &str, skills: &[Skill]) -> Framed {
    let body = match skills.is_empty() {
        true => task.to_string(),
        false => {
            let rows: Vec<String> = skills
                .iter()
                .map(|s| {
                    let line = s.description.lines().next().unwrap_or("").trim();
                    format!(
                        "- {} \u{2014} {} ({})",
                        s.name,
                        clip(line),
                        s.path.display()
                    )
                })
                .collect();
            format!(
                "Skills on hand \u{2014} playbooks for this machine. When one fits the task, \
                 read its file before you start and follow it; when none does, leave them \
                 be.\n{}\n\nTASK:\n{task}",
                rows.join("\n")
            )
        }
    };
    Framed {
        body,
        applied: Vec::new(),
    }
}

fn clip(line: &str) -> String {
    match line.chars().count() > LINE_CAP {
        true => format!(
            "{}\u{2026}",
            line.chars().take(LINE_CAP - 1).collect::<String>()
        ),
        false => line.to_string(),
    }
}

#[cfg(test)]
#[path = "skillcatalog_tests.rs"]
mod tests;
