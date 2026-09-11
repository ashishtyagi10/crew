//! What agent smith brings to a run, said once. The routing line says the
//! SHAPE; until this file nothing said what the run was starting WITH — that
//! two earlier turns ride in front of the task, that a saved note stands over
//! it, that a playbook was chosen, that forty tools are on the table, that the
//! tree it may touch already differs from HEAD. Each of those changes what the
//! model sees, and each was invisible until the run did something surprising.
//! So one quiet line after the routing line — `context: 2 earlier turns · a
//! note · skill code-review · 41 tools · tree dirty (3 files)` — only the
//! parts that are non-empty, and NOTHING when all are: a fresh session on a
//! clean tree with a small tool surface says nothing.
//!
//! This file REPORTS; it decides nothing. Every fact is read from where the
//! run itself reads it: the thread, the memory files, the skill decider (the
//! same pick the arm will make on the same text, so its memo answers and the
//! model is asked once), the tool catalog against `toolpick::BUDGET`, and the
//! router's `World`.
use crate::broker::relay::msg;
use crate::broker::session::Session;
use crate::PluginEvent;

use super::decision::SMITH;
use super::world::World;
use super::Shape;

/// The facts, counted. A zero or an empty list is a part not said.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ContextLine {
    /// Turns the thread carries in front of the task.
    pub(crate) turns: usize,
    /// Non-blank lines of standing memory (user file + project file).
    pub(crate) notes: usize,
    /// The playbooks the arm will frame, by name.
    pub(crate) skills: Vec<String>,
    /// Tools on the table — said only past the choosing budget, where the
    /// model picks a subset (the `tools: chose …` line follows later).
    pub(crate) tools: usize,
    /// Paths that differ from HEAD.
    pub(crate) dirty: usize,
}

impl ContextLine {
    /// The line for this session and world, or `None` when there is nothing
    /// to say. `skills` is the arm's pick, decided by [`skills_for`].
    pub(crate) fn gather(session: &Session, world: &World, skills: &[String]) -> Option<String> {
        Self::read(session, world, skills).line()
    }

    fn read(session: &Session, world: &World, skills: &[String]) -> Self {
        ContextLine {
            turns: crate::broker::thread::lock(&session.thread).len(),
            notes: crate::broker::memory::load()
                .map_or(0, |m| m.lines().filter(|l| !l.trim().is_empty()).count()),
            skills: skills.to_vec(),
            tools: session.tools().map_or(0, |t| t.specs().len()),
            dirty: world.dirty.unwrap_or(0),
        }
    }

    /// `context: <parts joined by · >`, or `None` when every part is empty.
    pub(crate) fn line(&self) -> Option<String> {
        let mut parts: Vec<String> = Vec::new();
        if self.turns > 0 {
            parts.push(format!(
                "{} earlier {}",
                self.turns,
                plural(self.turns, "turn")
            ));
        }
        match self.notes {
            0 => {}
            1 => parts.push("a note".into()),
            n => parts.push(format!("{n} notes")),
        }
        if !self.skills.is_empty() {
            parts.push(format!("skill {}", self.skills.join(", ")));
        }
        if self.tools > crate::broker::toolpick::BUDGET {
            parts.push(format!("{} tools", self.tools));
        }
        if self.dirty > 0 {
            parts.push(format!(
                "tree dirty ({} {})",
                self.dirty,
                plural(self.dirty, "file")
            ));
        }
        if parts.is_empty() {
            return None;
        }
        Some(format!("context: {}", parts.join(" \u{b7} ")))
    }
}

/// `turn`/`turns` — the ending the count wants.
fn plural(n: usize, one: &str) -> String {
    format!("{one}{}", if n == 1 { "" } else { "s" })
}

/// The playbooks `shape`'s arm will frame for `task`, decided now so the
/// line can name them. It is the arm's own call on the arm's own text, so
/// the decider's memo hands the arm the answer and the model is asked once —
/// which is why only the arms that frame the RAW task are asked here: the
/// swarm, a loop's or goal's first round, and a reply on an empty thread
/// (the relay wraps the thread's turns around the task first, and a
/// different text is a different question). The fan and the plan frame no
/// skills; the git shapes carry no task.
pub(crate) fn skills_for(shape: Shape, task: &str, session: &Session) -> Vec<String> {
    let raw = match shape {
        Shape::Swarm | Shape::Loop | Shape::Goal => true,
        Shape::Reply => crate::broker::thread::lock(&session.thread).len() == 0,
        _ => false,
    };
    if !raw {
        return Vec::new();
    }
    let skills = crate::broker::skills::load();
    crate::broker::skillchoice::decider()
        .pick(task, &skills)
        .skills
        .iter()
        .map(|s| s.name.clone())
        .collect()
}

/// Say the context line for the run `shape` is about to start, if there is
/// one — after the routing line, before the arm's first event.
pub(crate) fn announce(
    shape: Shape,
    task: &str,
    session: &Session,
    world: &World,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let skills = skills_for(shape, task, session);
    match ContextLine::gather(session, world, &skills) {
        Some(line) => emit(msg(SMITH, line)),
        None => Ok(()),
    }
}
