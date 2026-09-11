//! The routing decision, said out loud. Until this file the classifier was
//! silent: up to 30 s of dead air while it ran, then the chosen arm just
//! started — so a mis-route looked like a bug and a slow classifier looked
//! like a hang. Now every plain message gets ONE quiet line from agent smith
//! before dispatch — `routing: swarm — multi-part work` — naming the shape
//! and, when the model offered one, its reason; and every way the classifier
//! can stop (off, error, off-grammar) is said just as plainly instead of
//! silently becoming the swarm.
//!
//! The grammar grows an OPTIONAL second line, `WHY: <one short clause>`,
//! and two optional sizing lines after it (`ROUNDS:`, `AGENTS:` — see
//! `hints`). [`parse_decision_on`] is exactly as conservative as `parse_shape`:
//! a bad or missing extra line only drops that extra, never the shape.
use crate::broker::relay::msg;
use crate::broker::route::clip;
use crate::PluginEvent;

use super::hints::Hints;
use super::world::World;
use super::{classify, parse_shape, Classifier, Shape};

/// The routing line's sender — the same voice as the swarm's plan line, so
/// the pane draws it as a muted status line, not an agent speaking.
pub(super) const SMITH: &str = "agent smith";

/// The reason clause is one short line in the pane; a model that rambles
/// gets cut, not a wrapped card.
const WHY_MAX: usize = 80;

/// A parsed classifier reply: the shape, the model's reason when the
/// optional second line carried one, and its sizing hints when it gave any.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(crate) shape: Shape,
    pub(crate) why: Option<String>,
    pub(crate) hints: Hints,
}

impl Decision {
    /// The shape as the pane names it, sized: `loop ×5`, `fan → coder,
    /// reviewer`, or the bare name when the defaults apply.
    pub(crate) fn label(&self) -> String {
        format!("{}{}", self.shape.name(), self.hints.suffix())
    }
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
    /// The whole decision to dispatch: the model's, or a default-sized swarm
    /// for every stop.
    pub(crate) fn decision(&self) -> Decision {
        match self {
            Routing::Chosen(d) => d.clone(),
            _ => Decision::default(),
        }
    }

    /// The pane line: `routing: <shape>`, then ` — <reason>` when there is
    /// one to give (the model's clause, or the honest name of the stop).
    pub(crate) fn line(&self) -> String {
        let shape = self.decision().label();
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

/// Classify `task` in `world` and say the decision: a `thinking` activity
/// for agent smith while the classifier runs (the pane's header pulse — the
/// only state it draws live), the routing line, then smith's own idle. The
/// idle is ours to send: no dispatch arm ever settles agent smith, and the
/// turn-level idle comes minutes later.
pub(crate) fn announce(
    task: &str,
    world: &World,
    classifier: Option<Classifier>,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<Decision> {
    emit(PluginEvent::Activity {
        agent: SMITH.into(),
        state: "thinking".into(),
        from: "user".into(),
    })?;
    let routing = decide_in(task, world, classifier);
    emit(msg(SMITH, routing.line()))?;
    emit(PluginEvent::Activity {
        agent: SMITH.into(),
        state: "idle".into(),
        from: String::new(),
    })?;
    Ok(routing.decision())
}

/// Run the classifier and keep the WHOLE outcome, not just the shape: the
/// line needs to tell a call error from an off-grammar reply. The world's
/// roster is also the set an `AGENTS:` line may name — the model can pick
/// only from what it was shown.
pub(crate) fn decide_in(task: &str, world: &World, classifier: Option<Classifier>) -> Routing {
    let Some(call) = classifier else {
        return Routing::Off;
    };
    match call(&classify::prompt(task, world)) {
        Ok(reply) => {
            parse_decision_on(&reply, &world.agents).map_or(Routing::OffGrammar, Routing::Chosen)
        }
        Err(e) => Routing::Failed(e),
    }
}

/// Parse the reply against the grammar: `SHAPE:` on the first line (via
/// `parse_shape`, so the shape rules are defined once), an optional
/// `WHY: <clause>` on the second, and the optional sizing lines anywhere
/// after (see `Hints::parse`; `roster` is the set an `AGENTS:` line may
/// name — empty, and it names nobody). A second line that is not a WHY, or
/// an empty one, drops the reason and nothing else.
pub(crate) fn parse_decision_on(reply: &str, roster: &[String]) -> Option<Decision> {
    let shape = parse_shape(reply)?;
    let why = reply
        .trim()
        .lines()
        .nth(1)
        .and_then(|l| l.trim().split_once(':'))
        .filter(|(head, _)| head.trim().eq_ignore_ascii_case("why"))
        .map(|(_, tail)| clip(tail.trim().trim_end_matches('.'), WHY_MAX))
        .filter(|w| !w.is_empty());
    let hints = Hints::parse(reply, roster).relevant_to(shape);
    Some(Decision { shape, why, hints })
}
