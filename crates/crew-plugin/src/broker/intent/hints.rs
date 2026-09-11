//! The model's say over HOW MUCH and WHO — two optional grammar lines after
//! `SHAPE:`/`WHY:`. Until this file every loop ran exactly `LOOP_ROUNDS`,
//! every goal exactly `GOAL_ROUNDS`, and every fan went to the whole roster:
//! the model chose the shape and a constant chose everything else. Now
//! `ROUNDS: <n>` and `AGENTS: <name, name>` let the model size the work,
//! and the constants become what they should have been — backstops.
//!
//! A third line, `VERIFY: yes`, is the model's say over whether the result
//! needs CHECKING: a swarm (or a plan, which becomes one) for a request with
//! a checkable success condition is judged once it finishes and sent back
//! once when the judge says no (see `swarmverify`). The model decides — no
//! word list here guesses what "make the tests pass" means.
//!
//! Parsing is exactly as conservative as the shape line: junk drops the
//! hint and nothing else, an over-large count is clamped to the ceiling, an
//! unknown agent is dropped (exact name match against the roster the model
//! was shown — never fuzzy), and no line can ever change the shape.
use crate::broker::roundloop::MAX_ROUNDS;

use super::Shape;

/// What the model asked for beyond the shape. `None` everywhere means the
/// defaults — the caller's constants — apply.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Hints {
    /// Rounds for `loop`/`goal`, already clamped to `1..=MAX_ROUNDS`.
    pub(crate) rounds: Option<u32>,
    /// The fan subset, in roster spelling; `None` is the whole roster.
    pub(crate) agents: Option<Vec<String>>,
    /// The swarm's result is judged against the request when it finishes.
    pub(crate) verify: bool,
}

impl Hints {
    /// Read the optional lines from anywhere after the first line. `roster`
    /// is the set of names an `AGENTS:` line may pick from; an empty roster
    /// accepts nobody, so the subset falls back to everyone.
    pub(crate) fn parse(reply: &str, roster: &[String]) -> Hints {
        let mut hints = Hints::default();
        for line in reply.trim().lines().skip(1) {
            let Some((head, tail)) = line.trim().split_once(':') else {
                continue;
            };
            match head.trim().to_ascii_lowercase().as_str() {
                "rounds" => hints.rounds = parse_rounds(tail),
                "agents" => hints.agents = parse_agents(tail, roster),
                "verify" => hints.verify = parse_yes(tail),
                _ => {}
            }
        }
        hints
    }

    /// Keep only the hints `shape` can use: a count means nothing to a fan,
    /// a subset nothing to a loop, a check nothing to a reply (a goal has its
    /// own judge). Done once here so the pane line and the dispatch never
    /// disagree about what was chosen.
    pub(crate) fn relevant_to(self, shape: Shape) -> Hints {
        Hints {
            rounds: self
                .rounds
                .filter(|_| matches!(shape, Shape::Loop | Shape::Goal)),
            agents: self.agents.filter(|_| shape == Shape::Fan),
            verify: self.verify && matches!(shape, Shape::Swarm | Shape::Plan),
        }
    }

    /// The pane's suffix after the shape name: ` ×5` for a round count,
    /// ` → coder, reviewer` for a subset, ` · verified` for a checked
    /// result; empty when the defaults apply.
    pub(crate) fn suffix(&self) -> String {
        let mut s = String::new();
        if let Some(n) = self.rounds {
            s.push_str(&format!(" \u{00d7}{n}"));
        }
        if let Some(names) = &self.agents {
            s.push_str(&format!(" \u{2192} {}", names.join(", ")));
        }
        if self.verify {
            s.push_str(" \u{00b7} verified");
        }
        s
    }
}

/// `yes` (any case, trailing punctuation tolerated) and nothing else: the
/// model either asked for a check or it did not, and a hedge is a no.
fn parse_yes(tail: &str) -> bool {
    tail.split_whitespace()
        .next()
        .map(|t| t.trim_matches(|c: char| !c.is_ascii_alphabetic()))
        .is_some_and(|t| t.eq_ignore_ascii_case("yes"))
}

/// The first integer token, clamped into the loop's legal range. `0` becomes
/// one round rather than none: the model asked for work, not for silence.
fn parse_rounds(tail: &str) -> Option<u32> {
    let token = tail.split_whitespace().next()?;
    let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
    let n: u32 = digits.parse().ok()?;
    Some(n.clamp(1, MAX_ROUNDS))
}

/// Names split on commas, `+` or spaces, kept when they are on the roster
/// (case-insensitive exact match, returned in the roster's spelling, first
/// mention wins). `None` when nothing survives — the whole roster fans.
fn parse_agents(tail: &str, roster: &[String]) -> Option<Vec<String>> {
    let mut picked: Vec<String> = Vec::new();
    for raw in tail.split(|c: char| c == ',' || c == '+' || c.is_whitespace()) {
        let name = raw.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
        if name.is_empty() {
            continue;
        }
        if let Some(known) = roster.iter().find(|r| r.eq_ignore_ascii_case(name)) {
            if !picked.contains(known) {
                picked.push(known.clone());
            }
        }
    }
    (!picked.is_empty()).then_some(picked)
}
