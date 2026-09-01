//! Choosing which tools an agent is shown — retrieval, not concatenation.
//!
//! `SessionTools::hint` used to paste EVERY tool on EVERY connected MCP server into the task
//! body, on every hop, for every agent. That is free at four tools and a wall at forty: one
//! Google Workspace server is fifty on its own, and the token bill is the lesser half of the
//! problem. Selection ACCURACY collapses first — a model shown two hundred similarly-worded
//! one-line descriptions picks worse than one shown the twenty that could plausibly matter.
//!
//! So above a budget, the task decides what is shown. Three rules keep that honest, and they are
//! the whole design:
//!
//! 1. **Below the budget nothing is filtered.** A crew with ten tools behaves exactly as it did.
//! 2. **crew's own `sys` tools are never dropped.** They are a handful, they are how an agent
//!    does anything at all on this machine, and no phrasing makes them irrelevant.
//! 3. **Nothing is unreachable.** What was left out is COUNTED in the prompt, and
//!    `sys:find_tools` searches the whole catalog — so a tool that scored badly is one question
//!    away rather than gone. The invariant the goal states ("a tool the agent needs is never the
//!    one that got filtered out") cannot be met by a scorer alone; it is met by leaving a door.
use crate::mcp::McpTool;

/// How many tools a task's prompt may name. Chosen from the failure it prevents rather than from
/// a token count: a list a person could skim is a list a model can choose from.
pub(crate) const BUDGET: usize = 24;

/// Words that say nothing about which tool is wanted.
const STOP: &[&str] = &[
    "the", "and", "for", "with", "from", "this", "that", "what", "when", "how", "please", "can",
    "you", "your", "our", "are", "was", "were", "has", "have", "had", "will", "would", "should",
    "could", "into", "onto", "out", "get", "got", "put", "use", "using", "make", "made", "then",
    "than", "them", "they", "there", "here", "about", "all", "any", "some", "one", "two", "new",
    "old", "run", "runs", "let", "its", "not", "but", "who", "why", "may", "might", "must",
];

/// The words of `text` worth matching on: lowercase, three characters or more, not a stopword.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|w| w.len() >= 3 && !STOP.contains(&w.as_str()))
        .collect()
}

/// How well `tool` matches `task`. Higher is better; zero means nothing in common.
///
/// The weights say what a name is worth against what a description is worth: a tool CALLED
/// `calendar` is a better answer to "my calendar" than one that mentions calendars in passing.
pub(crate) fn score(tool: &McpTool, task_words: &[String], task_lower: &str) -> u32 {
    // Named outright — `@tool gcal:events` in the task, or the bare tool name as a word. This
    // is the one case where the answer is not a guess, so it outranks everything.
    if task_lower.contains(&format!("{}:{}", tool.server, tool.name).to_lowercase()) {
        return 1_000;
    }
    let name_words = words(&tool.name);
    let server_words = words(&tool.server);
    // The first line only: `McpTool::description` can be a server's whole paragraph, and a long
    // one would otherwise out-score a short precise name by sheer surface area.
    let desc_words = words(tool.description.lines().next().unwrap_or(""));
    let mut score = 0;
    for w in task_words {
        if name_words.contains(w) {
            score += 6;
        }
        if server_words.contains(w) {
            score += 4;
        }
        if desc_words.contains(w) {
            score += 1;
        }
    }
    score
}

/// The tools to show for `task`, and how many were left out.
///
/// Ordering is stable: score first, then crew's spelling, so the same task twice produces the
/// same prompt and a cached provider response stays cached.
pub(crate) fn pick(tools: Vec<McpTool>, task: &str, budget: usize) -> (Vec<McpTool>, usize) {
    if tools.len() <= budget {
        return (tools, 0);
    }
    let task_lower = task.to_lowercase();
    let task_words = words(task);
    let (mut kept, rest): (Vec<McpTool>, Vec<McpTool>) =
        tools.into_iter().partition(|t| t.server == "sys");
    let mut scored: Vec<(u32, String, McpTool)> = rest
        .into_iter()
        .map(|t| {
            let s = score(&t, &task_words, &task_lower);
            let label = format!("{}:{}", t.server, t.name);
            (s, label, t)
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let room = budget.saturating_sub(kept.len());
    let left_out = scored.len().saturating_sub(room);
    kept.extend(scored.into_iter().take(room).map(|(_, _, t)| t));
    (kept, left_out)
}

/// The line that admits what was left out. Empty when nothing was.
///
/// It says the NUMBER, because "some tools are hidden" is a sentence a model cannot act on, and
/// it names the way back to them.
pub(crate) fn omitted_note(left_out: usize) -> String {
    if left_out == 0 {
        return String::new();
    }
    format!(
        "\n({left_out} more tool(s) are connected but not listed here \u{2014} \
         call `sys:find_tools` with {{\"q\": \"what you need\"}} to search them by name \
         and description.)"
    )
}

/// The result of `sys:find_tools`: every tool matching `q`, best first.
pub(crate) fn search(tools: &[McpTool], q: &str, limit: usize) -> String {
    // An empty query is not "everything": a substring test against `""` matches every tool, and
    // answering a malformed call with the whole catalog is how the budget gets undone by
    // accident. Say what there is and let the model ask again with a word in it.
    if q.trim().is_empty() {
        return format!(
            "no tool matches {q:?}. {} tools are connected.",
            tools.len()
        );
    }
    let q_lower = q.to_lowercase();
    let q_words = words(q);
    let mut hits: Vec<(u32, String)> = tools
        .iter()
        .filter_map(|t| {
            // A bare substring match counts too: a search for "cal" should find `gcal:events`
            // even though "cal" is not a word of it.
            let label = format!("{}:{}", t.server, t.name);
            let s = score(t, &q_words, &q_lower);
            let substr = label.to_lowercase().contains(&q_lower)
                || t.description.to_lowercase().contains(&q_lower);
            (s > 0 || substr).then(|| {
                let one = t.description.lines().next().unwrap_or("");
                (
                    s + u32::from(substr),
                    format!("- {label} \u{2014} {}", super::route::clip(one, 100)),
                )
            })
        })
        .collect();
    if hits.is_empty() {
        return format!(
            "no tool matches {q:?}. {} tools are connected.",
            tools.len()
        );
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let shown = hits.len().min(limit);
    let mut out = hits
        .into_iter()
        .take(shown)
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n");
    out.push_str("\ncall one with `@tool <server>:<tool> {\"arg\": \u{2026}}`.");
    out
}

#[cfg(test)]
#[path = "toolpick_tests.rs"]
mod tests;
