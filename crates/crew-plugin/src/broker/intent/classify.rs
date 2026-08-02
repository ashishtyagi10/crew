//! The live classification call: one cheap, bounded completion on the
//! discovered provider. Kept apart from the routing so `mod.rs` stays the
//! shape logic and this file stays the provider plumbing.
use std::sync::Arc;
use std::time::Duration;

/// Output-token ceiling for the classification call: the grammar is one line.
const INTENT_MAX_TOKENS: u32 = 64;

/// Round-trip ceiling for classification — deliberately far below
/// `call_timeout()` (3 min): the router is overhead before the real work, so
/// a slow classifier must degrade to the swarm, not stall the task.
const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(30);

/// The live classifier, when one may run: `None` under `CREW_INTENT=0`, with
/// no resolvable provider, or under the mock provider (the GUI harness needs
/// deterministic swarm replies; a mock reply would fail the grammar anyway).
/// `pub(crate)` because `broker::elect` makes its agent-election call through
/// the same plumbing — one bounded structured call, one escape hatch.
pub(crate) fn live_classifier() -> Option<impl Fn(&str) -> Result<String, String>> {
    if super::disabled() {
        return None;
    }
    let (provider, model) = crate::broker::discover::provider_and_model()?;
    if model == "mock" {
        return None;
    }
    Some(move |p: &str| complete_once(&provider, &model, p))
}

/// One bounded completion on the discovered provider — same block-on pattern
/// as `ask::suggest_far_command` (a small one-shot needs its own max_tokens,
/// which the `Adapter` layer doesn't expose).
fn complete_once(
    provider: &Arc<dyn crew_hive::Provider>,
    model: &str,
    prompt: &str,
) -> Result<String, String> {
    let req = crew_hive::CompletionRequest {
        model: model.to_string(),
        system: None,
        prompt: prompt.to_string(),
        max_tokens: INTENT_MAX_TOKENS,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let fut = provider.complete(req);
    match rt.block_on(async move { tokio::time::timeout(CLASSIFY_TIMEOUT, fut).await }) {
        Ok(Ok(c)) => Ok(c.text),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!(
            "intent classification timed out after {CLASSIFY_TIMEOUT:?}"
        )),
    }
}

/// The classification prompt: one flat grammar over the execution shapes and
/// the capability intents; first match wins.
pub(super) fn prompt(task: &str) -> String {
    format!(
        "You route a user's message to ONE execution shape:\n\
         reply — a single agent answers or does it directly in one turn\n\
         fan — every agent tackles the same thing independently (the user wants many takes)\n\
         loop — one result refined over several rounds (iterate/polish/keep improving)\n\
         plan — draft a plan for approval before anything runs\n\
         goal — keep working in rounds until a stated success condition is judged met\n\
         swarm — multi-part work worth decomposing into parallel tasks\n\
         commit — draft a git commit message for the working diff (creating the commit still \
         waits for the user's confirm)\n\
         review — code-review the working diff, findings worst-first\n\
         standup — summarize recent commits as a standup update\n\
         resume — restore the previous session's conversation as context\n\
         The FIRST line of your reply must be exactly \
         `SHAPE: <reply|fan|loop|plan|goal|swarm|commit|review|standup|resume>`.\n\n\
         Message: {task}"
    )
}
