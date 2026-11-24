//! The Anthropic chain's rules, apart from its walk (`anthropicchain`).
//!
//! Which rungs follow an ask ([`tail`]):
//! - with `CREW_ANTHROPIC_MODEL` set, the LIST decides: the models after the
//!   ask's place in it, or, for an ask the list does not hold (a swarm's tier
//!   model, the advisor's Opus), the list itself, less any rung of a pricier
//!   family than the ask. A list chosen to avoid a model is never walked
//!   around by the built-in ladder;
//! - with no list, only the built-in ladder's own ids step, and only down:
//!   `claude-opus-5-5` → `claude-sonnet-5-5` → `claude-haiku-5-5`. Any other
//!   id (a pinned `claude-opus-4-1`, a dated snapshot) has no rung after it:
//!   it was pinned on purpose, and a silent jump to another generation would
//!   answer as a model nobody chose.
//!
//! Which failures walk ([`walks_on`]): 529 `overloaded_error`, 404
//! `not_found_error`, 429 `rate_limit_error` (Anthropic limits each model on
//! its own, so a sibling can still answer) and 5xx `api_error`, each after the
//! provider's own retries. Never one that is the key's or the account's
//! ([`account_wide`]), nor any other 4xx: a bad request, a permission or a
//! billing refusal would meet every rung again.
use super::fellback::account_wide;
use super::ProviderError;
use crate::ModelTier;

/// The built-in ladder, priciest first: the tiers' own ids.
const LADDER: [ModelTier; 3] = [ModelTier::Capable, ModelTier::Standard, ModelTier::Cheap];

/// The rungs after `model` (the module doc has the rule). Never ABOVE the
/// ask unless a list puts it there: one provider serves every tier, and a
/// cheap ask that fell up to Opus would be billed at Opus without anyone
/// choosing it.
pub(super) fn tail(model: &str, chain: &[String]) -> Vec<String> {
    if let Some(at) = chain.iter().position(|m| m == model) {
        return chain[at + 1..].to_vec();
    }
    if !chain.is_empty() {
        let pricier = |m: &&String| matches!((rank(m), rank(model)), (Some(r), Some(a)) if r > a);
        return chain.iter().filter(|m| !pricier(m)).cloned().collect();
    }
    let ladder = LADDER.map(|t| t.model_id());
    match ladder.iter().position(|m| *m == model) {
        Some(at) => ladder[at + 1..].iter().map(|m| m.to_string()).collect(),
        None => Vec::new(),
    }
}

/// A model's family, cheapest lowest; `None` for a name that names none.
fn rank(model: &str) -> Option<usize> {
    ["haiku", "sonnet", "opus"]
        .iter()
        .position(|f| model.contains(f))
}

/// Whether `e` is the MODEL's failure, one the next rung may not share:
/// overloaded (529), rate-limited (429: Anthropic limits each model on its
/// own), gone (404, a retired or mistyped id) or a host error (5xx) that
/// outlived its retries. A bad request, a refused key, a permission or
/// billing refusal, an unreachable host: every rung would meet it again.
pub(super) fn walks_on(e: &ProviderError) -> bool {
    if account_wide(e) {
        return false;
    }
    match e {
        ProviderError::Api(body) => {
            let v: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
            let kind = v["error"]["type"].as_str().unwrap_or_default();
            [
                "overloaded_error",
                "not_found_error",
                "rate_limit_error",
                "api_error",
            ]
            .contains(&kind)
        }
        ProviderError::Status(s) => {
            super::status::code(s).is_some_and(|c| c == 404 || c == 429 || c >= 500)
        }
        _ => false,
    }
}
