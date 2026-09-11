//! The model tier the swarm serves on — planner, workers and re-planner —
//! and the one place `CREW_SWARM_TIER` is read.
//!
//! WHY: `discover::provider_and_model()` resolves at `Cheap` because its
//! first callers were one-line asks — the Far pane's `!` hint, the intent
//! classifier, the election vote — where a small model is the right spend.
//! The swarm inherited that default by accident: on Anthropic every plan and
//! every worker ran on Haiku while the relay roster, the same seats asked
//! directly, served on Sonnet — and the footer said Sonnet. The swarm is the
//! brain; it serves at `Standard`. The bounded structured one-shots stay
//! Cheap on purpose.
//!
//! `CREW_SWARM_TIER=cheap` is the escape hatch, for a run that is about
//! breadth rather than judgement and where the bill matters more — a
//! Standard swarm costs roughly three times a Cheap one per token. Anything
//! else, including unset, is `Standard`: the knob can only make crew cheaper.
use crew_hive::ModelTier;

/// The tier this process's swarms serve on.
pub(crate) fn swarm_tier() -> ModelTier {
    parse(std::env::var("CREW_SWARM_TIER").ok().as_deref())
}

/// `raw` as a tier: `cheap` (any case, spaces around it) is `Cheap`; all
/// else is `Standard`.
pub(crate) fn parse(raw: Option<&str>) -> ModelTier {
    match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
        Some("cheap") => ModelTier::Cheap,
        _ => ModelTier::Standard,
    }
}

#[cfg(test)]
#[path = "swarmtier_tests.rs"]
mod tests;
