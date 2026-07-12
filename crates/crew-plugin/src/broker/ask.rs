//! One-shot "ask the AI for a command": powers the input bar's `?` prefix
//! (à la Warp AI / GitHub Copilot CLI) via `suggest_command`, and the Far
//! pane's `!` command bar suggestion via `suggest_far_command`. The former
//! reuses the broker's full `Adapter`/roster stack (`discover::roster_with`)
//! so it works wherever `/crew`'s inbuilt agents do; the latter calls
//! `discover::provider_and_model` directly, bypassing the `Adapter` layer,
//! because it needs a custom cwd/OS-aware system prompt and a small
//! `max_tokens` the roster's fixed-role adapters don't expose. Both are
//! blocking: call them from a worker thread, never the render thread.
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

/// Output-token ceiling for the Far pane's `!` one-shot ask: the model must
/// reply with a single command line, not a paragraph — small keeps latency
/// and cost down too.
const FAR_MAX_TOKENS: u32 = 128;

/// Translate `query` into exactly one POSIX shell command that will run in
/// `cwd` on this OS, via the discovered provider. Same discovery/mock rules
/// as [`suggest_command`] (`CREW_BROKER_MOCK_REPLY`, `CREW_PROVIDER`,
/// DASHSCOPE/OPENROUTER/ANTHROPIC auto-order, shell-env hydration) — but
/// calls the provider directly with a cwd/OS-aware system prompt and a small
/// `max_tokens`, bypassing the `Adapter`/roster layer entirely (see the
/// module doc comment for why). Used by the Far pane's `!` ask (crew-app's
/// `farpane` module); call from a worker thread, never the render thread.
pub fn suggest_far_command(
    query: &str,
    cwd: &std::path::Path,
    timeout: Duration,
) -> Result<String, String> {
    if std::env::var("CREW_BROKER_MOCK_REPLY").is_err() {
        static HYDRATE: std::sync::Once = std::sync::Once::new();
        HYDRATE.call_once(super::shellenv::hydrate);
    }
    let (provider, model) = super::discover::provider_and_model().ok_or_else(|| {
        "no AI provider — set DASHSCOPE_API_KEY, OPENROUTER_API_KEY, or ANTHROPIC_API_KEY"
            .to_string()
    })?;
    let req = far_request(query, cwd, model);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let fut = provider.complete(req);
    match rt.block_on(async move { tokio::time::timeout(timeout, fut).await }) {
        Ok(Ok(c)) => Ok(extract_command(&c.text)),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!("ask timed out after {timeout:?}")),
    }
}

/// Build the `!` ask's completion request: the exact system prompt, the
/// user's description as the prompt body, and the 128-token ceiling. Split
/// out from [`suggest_far_command`] so `max_tokens`/`system` are directly
/// unit-testable without a provider round-trip.
fn far_request(query: &str, cwd: &std::path::Path, model: String) -> crew_hive::CompletionRequest {
    crew_hive::CompletionRequest {
        model,
        system: Some(far_system_prompt(cwd)),
        prompt: query.to_string(),
        max_tokens: FAR_MAX_TOKENS,
    }
}

/// The exact system prompt the Far pane's `!` ask sends: demand one bare
/// POSIX command, name the directory it will run in and the OS.
fn far_system_prompt(cwd: &std::path::Path) -> String {
    format!(
        "Reply with exactly one POSIX shell command for the user's request. \
         No prose, no code fences. The command runs in {} on {}.",
        cwd.display(),
        std::env::consts::OS
    )
}

#[cfg(test)]
#[path = "ask_tests.rs"]
mod tests;
