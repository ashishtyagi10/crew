//! The routing decision, said out loud. Until this file the classifier was
//! silent: up to 30 s of dead air while it ran, then the chosen arm just
//! started — so a mis-route looked like a bug and a slow classifier looked
//! like a hang. Now every plain message gets ONE quiet line from agent smith
//! before dispatch — `routing: swarm — multi-part work` — naming the shape
//! and, when the model offered one, its reason; and every way the classifier
//! can stop (off, error, off-grammar) is said just as plainly instead of
//! silently becoming the swarm.
//!
//! The grammar grows an OPTIONAL second line, `WHY: <one short clause>`.
//! [`parse_decision`] is exactly as conservative as `parse_shape`: a bad or
//! missing WHY only drops the reason, never changes the shape.
use crate::broker::relay::msg;
use crate::broker::route::clip;
use crate::PluginEvent;

use super::{classify, parse_shape, Classifier, Shape};

/// The routing line's sender — the same voice as the swarm's plan line, so
/// the pane draws it as a muted status line, not an agent speaking.
const SMITH: &str = "agent smith";

/// The reason clause is one short line in the pane; a model that rambles
/// gets cut, not a wrapped card.
const WHY_MAX: usize = 80;

/// A parsed classifier reply: the shape, plus the model's reason when the
/// optional second line carried one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(crate) shape: Shape,
    pub(crate) why: Option<String>,
}

/// How the router arrived at its shape. Every variant that is not `Chosen`
/// is the swarm — but each says WHY it is the swarm, so the fallback is
/// never mistaken for a choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Routing {
    Chosen(Decision),
    /// No classifier may run: `CREW_INTENT=0`, keyless, or the mock provider.
    Off,
    /// The call itself failed (transport, timeout).
    Failed(String),
    /// The model answered outside the `SHAPE:` grammar.
    OffGrammar,
}

impl Routing {
    /// The shape to dispatch — the pre-router swarm for every stop.
    pub(crate) fn shape(&self) -> Shape {
        match self {
            Routing::Chosen(d) => d.shape,
            _ => Shape::Swarm,
        }
    }

    /// The pane line: `routing: <shape>`, then ` — <reason>` when there is
    /// one to give (the model's clause, or the honest name of the stop).
    pub(crate) fn line(&self) -> String {
        let shape = self.shape().name();
        match self {
            Routing::Chosen(Decision { why: Some(why), .. }) => format!("routing: {shape} — {why}"),
            Routing::Chosen(_) => format!("routing: {shape}"),
            Routing::Off => format!("routing: {shape} — classifier off"),
            Routing::Failed(e) => {
                format!("routing: {shape} — classifier failed: {}", clip(e, WHY_MAX))
            }
            Routing::OffGrammar => format!("routing: {shape} — classifier reply was off-grammar"),
        }
    }
}

impl Shape {
    /// The grammar token — the one word the model said and the pane repeats.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Shape::Reply => "reply",
            Shape::Fan => "fan",
            Shape::Loop => "loop",
            Shape::Plan => "plan",
            Shape::Goal => "goal",
            Shape::Swarm => "swarm",
            Shape::Commit => "commit",
            Shape::Review => "review",
            Shape::Standup => "standup",
            Shape::Resume => "resume",
        }
    }
}

/// Classify `task` and say the decision: a `thinking` activity for agent
/// smith while the classifier runs (the pane's header pulse — the only state
/// it draws live), the routing line, then smith's own idle. The idle is ours
/// to send: no dispatch arm ever settles agent smith, and the turn-level idle
/// comes minutes later.
pub(crate) fn announce(
    task: &str,
    classifier: Option<Classifier>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<Shape> {
    emit(PluginEvent::Activity {
        agent: SMITH.into(),
        state: "thinking".into(),
        from: "user".into(),
    })?;
    let routing = decide(task, classifier);
    emit(msg(SMITH, routing.line()))?;
    emit(PluginEvent::Activity {
        agent: SMITH.into(),
        state: "idle".into(),
        from: String::new(),
    })?;
    Ok(routing.shape())
}

/// Run the classifier and keep the WHOLE outcome, not just the shape: the
/// line needs to tell a call error from an off-grammar reply.
pub(crate) fn decide(task: &str, classifier: Option<Classifier>) -> Routing {
    let Some(call) = classifier else {
        return Routing::Off;
    };
    match call(&classify::prompt(task)) {
        Ok(reply) => parse_decision(&reply).map_or(Routing::OffGrammar, Routing::Chosen),
        Err(e) => Routing::Failed(e),
    }
}

/// Parse the reply against the two-line grammar: `SHAPE:` on the first line
/// (via `parse_shape`, so the shape rules are defined once) and an optional
/// `WHY: <clause>` on the second. A second line that is not a WHY, or an
/// empty one, drops the reason and nothing else.
pub(crate) fn parse_decision(reply: &str) -> Option<Decision> {
    let shape = parse_shape(reply)?;
    let why = reply
        .trim()
        .lines()
        .nth(1)
        .and_then(|l| l.trim().split_once(':'))
        .filter(|(head, _)| head.trim().eq_ignore_ascii_case("why"))
        .map(|(_, tail)| clip(tail.trim().trim_end_matches('.'), WHY_MAX))
        .filter(|w| !w.is_empty());
    Some(Decision { shape, why })
}
