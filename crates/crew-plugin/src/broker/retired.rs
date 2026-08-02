//! The retirement table: every command the intent router replaced, with the
//! hint that teaches its plain-language phrasing. Typing an old slash form
//! answers instantly with the replacement ask — never a silent
//! reinterpretation, never a generic did-you-mean (`commands::handle`).

/// Commands retired in favor of the intent router: the capability lives on,
/// reached by plain language, and typing the old slash form teaches the
/// phrasing instantly (see `handle`).
pub(super) const RETIRED: &[(&str, &str)] = &[
    (
        "fan",
        "/fan is retired — just ask: \u{201c}have every agent take a crack at \u{2026}\u{201d}",
    ),
    (
        "loop",
        "/loop is retired — just ask: \u{201c}keep refining \u{2026} over a few rounds\u{201d}",
    ),
    (
        "commit",
        "/commit is retired — just ask: \u{201c}commit this\u{201d}; the draft then waits for \
         you to say \u{201c}apply\u{201d}",
    ),
    (
        "review",
        "/review is retired — just ask: \u{201c}look over my changes\u{201d}",
    ),
    (
        "standup",
        "/standup is retired — just ask: \u{201c}what did I ship this week?\u{201d}",
    ),
    (
        "resume",
        "/resume is retired — just ask: \u{201c}pick up where we left off\u{201d}",
    ),
    (
        "goal",
        "/goal is retired — just ask: \u{201c}keep working until \u{2026}\u{201d}; a judge \
         agent rules when the goal is met",
    ),
    (
        "plan",
        "/plan is retired — just ask: \u{201c}draft a plan for \u{2026}\u{201d}; the draft \
         then waits for your \u{201c}approve\u{201d} or \u{201c}reject\u{201d}",
    ),
    (
        "approve",
        "/approve is retired — with a plan pending, just say \u{201c}approve\u{201d} \
         (or \u{201c}run it\u{201d})",
    ),
    (
        "reject",
        "/reject is retired — with a plan pending, just say \u{201c}reject\u{201d} \
         (or \u{201c}drop it\u{201d})",
    ),
    (
        "skill",
        "/skill is retired — skills apply themselves: name one in your task (playbooks \
         still load from .crew/skills; ask \u{201c}what skills are loaded?\u{201d} to list them)",
    ),
    (
        "memory",
        "/memory is retired — just ask: \u{201c}what do you remember?\u{201d} \
         (#<note> still saves one)",
    ),
    (
        "mcp",
        "/mcp is retired — /doctor lists each server and its tools; @tool still calls them",
    ),
];
