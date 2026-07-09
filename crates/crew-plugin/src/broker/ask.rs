//! One-shot "ask the AI for a command": powers the input bar's `?` prefix
//! (à la Warp AI / GitHub Copilot CLI). Reuses the broker's full provider
//! stack — mock, DashScope, OpenRouter, Anthropic, per-provider fallback
//! chains — via `discover::roster_with`, so the ask works wherever `/crew`'s
//! inbuilt agents do, with zero duplicated provider code. Blocking: call it
//! from a worker thread, never the render thread.
use std::time::Duration;

/// Translate `query` (plain English) into one shell command via the discovered
/// provider. Returns the cleaned command, or a human-readable error for the
/// status line ("no AI provider — …", timeouts, HTTP failures).
pub fn suggest_command(query: &str, timeout: Duration) -> Result<String, String> {
    // Import missing provider keys from the login shell once (a Dock-launched
    // Crew has a bare env) — skipped under the mock, which needs no key.
    if std::env::var("CREW_BROKER_MOCK_REPLY").is_err() {
        static HYDRATE: std::sync::Once = std::sync::Once::new();
        HYDRATE.call_once(super::shellenv::hydrate);
    }
    let adapters = super::discover::roster_with(&std::collections::HashMap::new());
    // The coder role fits command synthesis; any adapter (a manifest plugin
    // agent, say) can answer when the inbuilt roster is empty.
    let adapter = adapters
        .iter()
        .find(|a| a.name() == "coder")
        .or_else(|| adapters.first())
        .ok_or_else(|| {
            "no AI provider — set DASHSCOPE_API_KEY, OPENROUTER_API_KEY, or ANTHROPIC_API_KEY"
                .to_string()
        })?;
    let reply = adapter.call(&ask_prompt(query), timeout)?;
    Ok(extract_command(&reply))
}

/// Explain a terminal pane's output (à la Warp's "ask AI about this"):
/// `context` is the pane's recent scrollback, `question` the user's ask (a
/// default stands in when blank). Returns a markdown answer for the viewer.
/// Same provider stack and threading rules as [`suggest_command`].
pub fn explain_output(context: &str, question: &str, timeout: Duration) -> Result<String, String> {
    if std::env::var("CREW_BROKER_MOCK_REPLY").is_err() {
        static HYDRATE: std::sync::Once = std::sync::Once::new();
        HYDRATE.call_once(super::shellenv::hydrate);
    }
    let adapters = super::discover::roster_with(&std::collections::HashMap::new());
    // The reviewer role fits post-mortems; any adapter can answer.
    let adapter = adapters
        .iter()
        .find(|a| a.name() == "reviewer")
        .or_else(|| adapters.first())
        .ok_or_else(|| {
            "no AI provider — set DASHSCOPE_API_KEY, OPENROUTER_API_KEY, or ANTHROPIC_API_KEY"
                .to_string()
        })?;
    adapter.call(&explain_prompt(context, question), timeout)
}

/// The explain prompt: the pane's output, the user's question (or a default),
/// and a concise-markdown answer format the `/md` viewer renders well.
fn explain_prompt(context: &str, question: &str) -> String {
    let q = question.trim();
    let q = if q.is_empty() {
        "Explain what happened here, focusing on any errors and how to fix them."
    } else {
        q
    };
    format!(
        "You are looking at the recent output of a user's terminal pane.\n\
         Answer their question concisely in markdown (short headings, code \
         fences for commands). Terminal output:\n\
         ```\n{context}\n```\n\
         Question: {q}"
    )
}

/// The single-completion prompt: name the platform, demand exactly one
/// command, ban prose and fences (models add them anyway — see
/// [`extract_command`]).
fn ask_prompt(query: &str) -> String {
    format!(
        "Translate the request into exactly ONE {} shell command.\n\
         Reply with only the command on a single line — no prose, no code \
         fences, no explanation.\n\
         Request: {query}",
        std::env::consts::OS
    )
}

/// Distill a model reply down to the bare command: prefer the first fenced
/// block's content, else the first non-empty line; strip inline backticks and
/// a leading `$ ` prompt.
pub(crate) fn extract_command(reply: &str) -> String {
    let mut cmd = "";
    let mut in_fence = false;
    for line in reply.lines() {
        let t = line.trim();
        if t.starts_with("```") {
            if in_fence {
                break;
            }
            in_fence = true;
            continue;
        }
        if in_fence && !t.is_empty() {
            cmd = t;
            break;
        }
    }
    if cmd.is_empty() {
        cmd = reply
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .unwrap_or("");
    }
    cmd.trim_start_matches("$ ").trim_matches('`').trim().into()
}

#[cfg(test)]
#[path = "ask_tests.rs"]
mod tests;
