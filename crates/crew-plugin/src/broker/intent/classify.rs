//! The live classification call: one cheap, bounded completion on the
//! discovered provider. Kept apart from the routing so `mod.rs` stays the
//! shape logic and this file stays the provider plumbing.
use std::sync::Arc;
use std::time::Duration;

/// Output-token ceiling for the classification call: the grammar is five
/// short lines at most (a shape, an optional one-clause reason, the
/// optional sizing lines, and the optional verify line).
const INTENT_MAX_TOKENS: u32 = 128;

/// Round-trip ceiling for the ROUTER's call. It answers in about a second
/// (measured on qwen-flash, DashScope's cheap tier), so 12 s is over ten
/// times its usual time. It was 30 s, shared with everything below, and a
/// hung router cost that before the turn fell into a whole swarm; now it
/// costs 12 s and lands on one agent (`decision::Routing::decision`).
pub(super) const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(12);

/// Round-trip ceiling for every OTHER one-shot on this plumbing: election,
/// skill and tool picks, `compact`'s summarizer, the judge, the swarm's
/// closing answer. Far below `call_timeout()` (3 min), since each is a step
/// around the work, but not the router's 12 s: the summarizer and the closing
/// answer write paragraphs, and a bound sized for five short lines would
/// fail them on an ordinary day.
pub(super) const ONESHOT_TIMEOUT: Duration = Duration::from_secs(30);

/// The router's own call: the grammar's output ceiling on the cheap tier,
/// under [`CLASSIFY_TIMEOUT`]. `pub(super)` so the tighter bound stays the
/// router's — nothing outside `intent` can pick it up by accident.
pub(super) fn live_router() -> Option<impl Fn(&str) -> Result<String, String>> {
    let call = live_bounded(
        INTENT_MAX_TOKENS,
        crew_hive::ModelTier::Cheap,
        CLASSIFY_TIMEOUT,
    )?;
    Some(move |p: &str| call(p).map(|c| c.text))
}

/// A grammar-sized call at the one-shot bound, when one may run: `None`
/// under `CREW_INTENT=0`, with no resolvable provider, or under the mock
/// provider. `broker::elect` makes its agent-election call through it — the
/// same small structured answer as routing, without the router's bound.
pub(crate) fn live_classifier() -> Option<impl Fn(&str) -> Result<String, String>> {
    live_call(INTENT_MAX_TOKENS)
}

/// The same bounded one-shot with a caller-chosen output ceiling — the shared
/// plumbing behind election, skill and tool picks, and `compact`'s
/// summarizer. One set of gates (`CREW_INTENT=0`, keyless, mock), one escape
/// hatch.
pub(crate) fn live_call(max_tokens: u32) -> Option<impl Fn(&str) -> Result<String, String>> {
    live_call_at(max_tokens, crew_hive::ModelTier::Cheap)
}

/// [`live_call`] on a stated tier. The quick decisions — a shape, a skill, a
/// tool, a summary — are `Cheap`; a call whose output IS the work the user
/// reads (the swarm's closing answer, a judge's verdict) is `Standard`, since
/// on DashScope `Cheap` became a smaller model (`discover::DASHSCOPE_CHEAP_MODEL`).
pub(crate) fn live_call_at(
    max_tokens: u32,
    tier: crew_hive::ModelTier,
) -> Option<impl Fn(&str) -> Result<String, String>> {
    let call = live_completion_at(max_tokens, tier)?;
    Some(move |p: &str| call(p).map(|c| c.text))
}

/// [`live_call_at`], keeping the whole reply — for a caller that has to know
/// whether the ceiling cut it (the swarm's closing answer says so).
pub(crate) fn live_completion_at(
    max_tokens: u32,
    tier: crew_hive::ModelTier,
) -> Option<impl Fn(&str) -> Result<crew_hive::Completion, String>> {
    live_bounded(max_tokens, tier, ONESHOT_TIMEOUT)
}

/// Every call here, with its round-trip bound stated: the gates live once,
/// and only the router passes its tighter bound.
fn live_bounded(
    max_tokens: u32,
    tier: crew_hive::ModelTier,
    timeout: Duration,
) -> Option<impl Fn(&str) -> Result<crew_hive::Completion, String>> {
    if super::disabled() {
        return None;
    }
    let (provider, model) = crate::broker::discover::provider_and_model_for(tier)?;
    if model == "mock" {
        return None;
    }
    Some(move |p: &str| complete_once(&provider, &model, p, max_tokens, timeout))
}

/// One bounded completion on the discovered provider — same block-on pattern
/// as `ask::suggest_far_command` (a small one-shot needs its own max_tokens,
/// which the `Adapter` layer doesn't expose).
fn complete_once(
    provider: &Arc<dyn crew_hive::Provider>,
    model: &str,
    prompt: &str,
    max_tokens: u32,
    timeout: Duration,
) -> Result<crew_hive::Completion, String> {
    let req = crew_hive::CompletionRequest {
        model: model.to_string(),
        system: None,
        prompt: prompt.to_string(),
        max_tokens,
        ..Default::default()
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let fut = provider.complete(req);
    // The caller names what timed out ("classifier failed: …"), so the
    // error is only the bound — the router's and the closing answer's alike.
    match rt.block_on(async move { tokio::time::timeout(timeout, fut).await }) {
        Ok(Ok(c)) => Ok(c),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!("timed out after {timeout:?}")),
    }
}

/// The classification prompt: one flat grammar over the execution shapes and
/// the capability intents; first match wins. The reason line is optional
/// and short on purpose — it is repeated verbatim in the pane. The order is
/// cache-aware, as `route::frame`'s is: the invariant grammar first, then
/// the world (changes per session), then the message (changes per call).
pub(super) fn prompt(task: &str, world: &super::world::World) -> String {
    let skills = match world.skills.is_empty() {
        true => "",
        false => super::skillhint::GRAMMAR,
    };
    let world = world.section();
    let max = crate::broker::roundloop::MAX_ROUNDS;
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
         `SHAPE: <reply|fan|loop|plan|goal|swarm|commit|review|standup|resume>`.\n\
         An optional second line `WHY: <one short clause>` says why, in ten words \
         or fewer. Two more optional lines size the work: `ROUNDS: <1-{max}>` \
         (loop or goal only — how many rounds it deserves; omit it for the \
         default) and `AGENTS: <name, name>` (fan: a subset of the agents \
         listed below, when fewer clearly fit; reply: the ONE agent whose role \
         fits the message best). One more optional line \
         `VERIFY: yes` (swarm or plan only) says the message states a checkable \
         success condition — tests passing, a build compiling, \"so that X\" — so \
         the result should be judged against it when the work ends; omit it when \
         there is nothing to check. A last optional line `TIER: cheap` (swarm, \
         plan or goal) asks for the small fast model instead of the standard \
         one — say it for mechanical breadth (rename, list, collect, summarise) \
         and omit it for anything needing judgement.{skills} Nothing else.\n\n\
         {world}Message: {task}"
    )
}
