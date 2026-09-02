//! The planner learns what the agents can reach.
//!
//! `PLANNER_SYSTEM` describes the shape of a plan and nothing about the world
//! the tasks will run in, so a goal like "brief me on tomorrow's weather" was
//! planned as research-by-recall even when a weather integration sat one
//! `@tool` away. The agents found it — the hint is per task — but a plan that
//! does not know a thing is reachable cannot put it in a task, and cannot
//! give that task to the specialist who would hold it.
//!
//! One line per source, appended to the system prompt. Nothing at all when
//! there are none: the prompt with no capabilities is byte-identical to the
//! prompt before this existed, so the A/B'd clauses above it keep the
//! measurements they were earned with.

/// The section to append for `caps`, or the empty string.
pub(crate) fn section(caps: &[String]) -> String {
    let lines: Vec<&str> = caps
        .iter()
        .map(|c| c.trim())
        .filter(|c| !c.is_empty())
        .collect();
    if lines.is_empty() {
        return String::new();
    }
    let mut out =
        String::from("\n\nThe agents can REACH these, through tools they call themselves:\n");
    for l in lines {
        out.push_str("- ");
        out.push_str(l);
        out.push('\n');
    }
    out.push_str(
        "A task that needs one of them is reachable, not research: say so in its \
         `prompt` (name the source), and give it to the specialist who would use it. \
         Do not invent sources that are not listed.",
    );
    out
}
