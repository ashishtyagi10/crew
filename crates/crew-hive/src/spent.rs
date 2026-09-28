//! What a model call cost, handed back beside what it said.
//!
//! WHY: a worker's spend reaches the host on the bus (`TokenDelta`,
//! `CostDelta`, one pair per round), and a swarm's footer was the sum of
//! those events and nothing else. The calls around the workers — the plan,
//! its repair re-ask, a re-plan — belong to no agent, so nothing published
//! them: their usage was dropped where their text was read, and on a small
//! plan they are a good part of the bill. A `Spent` is that usage, returned
//! with the result, for whoever sums the turn to add in.
use std::ops::AddAssign;

use crate::graph::ModelTier;
use crate::provider::Completion;

/// Tokens in and out, and micro-USD, of one call or of several added up.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Spent {
    pub input: u64,
    pub output: u64,
    pub micros_usd: u64,
}

impl Spent {
    /// `c`'s usage, billed exactly as a worker's round is: the provider's own
    /// figure when it gave one, else the price list at `model`, else at the
    /// `tier`'s own model — so a call around the work and the work itself
    /// are priced by one rule, not two that drift apart.
    pub fn billed(model: &str, tier: ModelTier, c: &Completion) -> Spent {
        Spent {
            input: u64::from(c.input_tokens),
            output: u64::from(c.output_tokens),
            micros_usd: crate::apiagent::billed(model, tier, c),
        }
    }

    /// Tokens in and out together — what the footer's meter counts.
    pub fn tokens(self) -> u64 {
        self.input + self.output
    }

    /// Nothing spent: no call made, or none that reported usage.
    pub fn is_zero(self) -> bool {
        self == Spent::default()
    }
}

impl AddAssign for Spent {
    fn add_assign(&mut self, other: Spent) {
        self.input += other.input;
        self.output += other.output;
        self.micros_usd += other.micros_usd;
    }
}

#[cfg(test)]
#[path = "spent_tests.rs"]
mod tests;
