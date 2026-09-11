//! The system prompt a planner-invented specialist runs under.
//!
//! The planner names a specialist per task (`specialty`, `expertise`), but
//! until now that name was a label: the swarm worker ran with no system
//! prompt at all, anonymous, while the relay path gave the same specialist a
//! real persona. The model's job is the best response per step, and a step
//! answered by "the security-auditor" reads differently from one answered by
//! nobody — so the identity the planner invented is what the worker is told
//! it is. Lives in crew-hive because the plan is parsed here; the relay's
//! adapter shares the opening line so the two paths never drift apart.

/// The opening of every specialist prompt: who it is, and what it is good
/// at. `role` may be empty (the planner is allowed to omit expertise), and
/// then the sentence about it is left out rather than read "specialty is .".
pub fn identity(name: &str, role: &str) -> String {
    if role.is_empty() {
        format!("You are the {name}.")
    } else {
        format!("You are the {name}. Your specialty is {role}.")
    }
}

/// The prompt for one swarm worker: [`identity`], then its place in the plan
/// — this task, by title, with the rest handled elsewhere — and the ask to
/// answer with the deliverable, not with talk about it. The plan's other
/// tasks and the dependency outputs already ride in the task body.
pub fn worker(specialty: &str, expertise: &str, title: &str) -> String {
    format!(
        "{} You are one worker in a larger plan: your task is \u{201c}{title}\u{201d}, \
         and other workers handle the rest, so do only this task and do it well. \
         Reply with the deliverable itself \u{2014} no preamble, no restating the \
         task, no commentary about the plan. Be concise.",
        identity(specialty, expertise)
    )
}

#[cfg(test)]
#[path = "persona_tests.rs"]
mod tests;
