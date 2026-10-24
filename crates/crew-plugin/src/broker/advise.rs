//! Where the advisor's model comes from. The advisor itself — a second model
//! reading a failed check, its words joining the turn — is the harness's now
//! (`crew_hive::advisor`), so a swarm worker's failed build is read as a
//! relay turn's is; what stays here is the host's half: the knobs, and the
//! provider found when a check first fails.
//!
//! `CREW_ADVISOR=0` turns it off; `CREW_ADVISOR_MODEL` names the model, else
//! it is the provider's capable tier. On DashScope that is the chain head,
//! often the executor's own model: a fresh read of the error alone is the
//! point, not a bigger model.
use std::sync::Arc;

use crate::broker::intent::classify::ONESHOT_TIMEOUT;
use crew_hive::advisor::{Ask, Asked, Find, Hooks};
use crew_hive::{CompletionRequest, ModelTier, Provider, Spent};

/// The tier the advisor runs on when `CREW_ADVISOR_MODEL` names none.
const TIER: ModelTier = ModelTier::Capable;

/// Output ceiling. The advice is three short lines, but Opus 5.5 always
/// thinks first, and its thinking is paid out of this.
const MAX_TOKENS: u32 = 1024;

/// How a turn finds its advisor's call: from the environment, once a check
/// has failed (finding the provider reads the credential store).
pub(crate) fn find() -> Find {
    Arc::new(live)
}

/// The advisor's one quiet line, in agent smith's name, the way a taken steer
/// is announced (`steer::say`): to the task on this thread.
pub(crate) fn say() -> Arc<dyn Fn(&str) + Send + Sync> {
    Arc::new(|line| crate::broker::steer::say(crate::broker::relay::msg("agent smith", line)))
}

/// The hooks a relay turn's advisor reports through: its line said, its cost
/// handed to `spent`.
pub(crate) fn hooks(spent: Arc<dyn Fn(Spent) + Send + Sync>) -> Hooks {
    Hooks { say: say(), spent }
}

/// Whether `CREW_ADVISOR` (passed in) leaves the advisor on: anything but `0`.
fn on(knob: Option<&str>) -> bool {
    knob.map(str::trim) != Some("0")
}

/// The advisor a turn may ask, from the environment.
#[cfg(not(test))]
fn live() -> Option<Ask> {
    let knob = std::env::var("CREW_ADVISOR").ok();
    let model = std::env::var("CREW_ADVISOR_MODEL").ok();
    let found = || crate::broker::discover::provider_and_model_for(TIER);
    resolve(knob.as_deref(), model, found)
}

/// Lib tests never reach a provider: a turn's advisor is the one its test put
/// on this thread, else none.
#[cfg(test)]
fn live() -> Option<Ask> {
    tests::installed()
}

/// The advisor on the provider `found` names, `model` overriding its model:
/// `None` when the knob is off (before anything is resolved), with no
/// provider, or under the mock. One plain completion: no tools, no system
/// prompt, nothing but the brief, at the one-shot bound the judges use.
fn resolve(
    knob: Option<&str>,
    model: Option<String>,
    found: impl FnOnce() -> Option<(Arc<dyn Provider>, String)>,
) -> Option<Ask> {
    if !on(knob) {
        return None;
    }
    let (provider, tier_model) = found().filter(|(_, m)| m != "mock")?;
    let model = model.filter(|m| !m.trim().is_empty()).unwrap_or(tier_model);
    Some(Arc::new(move |brief: String| -> Asked {
        let (provider, model) = (Arc::clone(&provider), model.clone());
        Box::pin(async move {
            let req = CompletionRequest {
                model: model.clone(),
                prompt: brief,
                max_tokens: MAX_TOKENS,
                ..Default::default()
            };
            match tokio::time::timeout(ONESHOT_TIMEOUT, provider.complete(req)).await {
                Ok(Ok(c)) => Ok((c.text.clone(), Spent::billed(&model, TIER, &c))),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err(format!("timed out after {ONESHOT_TIMEOUT:?}")),
            }
        })
    }))
}

#[cfg(test)]
#[path = "advise_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "adviseround_tests.rs"]
mod round_tests;

#[cfg(test)]
#[path = "adviselive_tests.rs"]
mod live_tests;

#[cfg(test)]
#[path = "advisehive_tests.rs"]
mod hive_tests;
