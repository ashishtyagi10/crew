//! The `/help` text — what the broker answers with, word for word.
//!
//! WHY its own file: `commands.rs` is over the line cap and must not grow,
//! and this text grows every time agent smith learns something. It carries
//! two kinds of line: the constructs (`/name — what it does`, pinned in both
//! directions by `commands_tests` — every construct the router answers is
//! here, and nothing here routes nowhere), and the prose about what the
//! brain DECIDES on a plain message — shape, size, who, whether the result
//! is checked, tools, skills, memory of the last turns, an approved plan
//! run as the swarm — so the pane's own help describes what smith does now,
//! not only the verbs.
/// One-line summaries of every construct, shown by `/help`.
pub(crate) const HELP: &str = "constructs:\n\
    /help — this list\n\
    /model — the roster with each agent's model (also in the pane footer)\n\
    /model <agent> <model|default> — pin an agent to a model (mix models freely)\n\
    /model all <model|default> — set every agent's model at once\n\
    /model <provider|n> — sign in with OAuth, no API key needed (the picker's top rows), \
    or make a signed-in provider serve\n\
    /logout — remove a stored OAuth sign-in: pick one from the popup (a key, if present, \
    serves again); /logout <name> removes it directly\n\
    plain language routes itself — \u{201c}have every agent take a crack at \u{2026}\u{201d} \
    fans out in parallel; \u{201c}keep refining \u{2026}\u{201d} runs improvement rounds; \
    \u{201c}keep working until \u{2026}\u{201d} loops with a judge until it rules the goal met; \
    \u{201c}draft a plan for \u{2026}\u{201d} waits for your \u{201c}approve\u{201d} or \
    \u{201c}reject\u{201d} before anything runs; \
    \u{201c}commit this\u{201d} drafts a commit message (say \u{201c}apply\u{201d} to create it); \
    \u{201c}look over my changes\u{201d}, \u{201c}what did I ship this week\u{201d} and \
    \u{201c}pick up where we left off\u{201d} reach code review, a standup and the last session\n\
    agent smith decides: the shape of every plain message (reply, fan, loop, goal, plan, \
    swarm \u{2014} or a commit, review, standup, resume) and says so on a routing line; how \
    many rounds a loop or goal gets and which agents a fan goes to; whether a judge verifies \
    the result; which tools a crowded task is shown and which skills it follows; it \
    remembers the last six turns, so \u{201c}shorter\u{201d} and \u{201c}now the tests \
    too\u{201d} mean what you meant, and an approved plan runs as the swarm. A context line \
    before each run says what it brings: earlier turns, notes, skills, tools, a dirty tree\n\
    /restore [n] — list the automatic snapshots, or put snapshot n's files back\n\
    /diff — everything different from the last commit as a patch, new files included \
    (every task that changes files shows its own patch and the diagnostics after it)\n\
    /doctor — health-check the AI stack (provider, CLIs, MCP servers and tools, memory, session)\n\
    #<note> — remember a preference (ask \u{201c}what do you remember?\u{201d} to see them)\n\
    skills: drop .md playbooks into .crew/skills — a task that names one applies it by itself\n\
    /reload — re-read skills, plugin agents, integrations and mcp.json without a restart\n\
    /stop [#n] — cancel all background tasks, or just task #n\n\
    @<agent> <task> — choose who starts the relay\n\
    @<a>+<b> <task> — those agents answer in parallel\n\
    \u{2026} tip: tasks run in the background — the footer lists them, /stop #n cancels one\n\
    \u{2026} tip: a model's reasoning streams live above its reply and folds when it lands \
    (CREW_THINKING=0 stops asking providers for it; CREW_STREAM_TEXT=0 stops all streaming)\n\
    aliases: /h /d /m /r\n\
    ";

#[cfg(test)]
#[path = "helptext_tests.rs"]
mod tests;
