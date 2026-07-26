//! HiveEvent → chat-facing PluginEvent translation for swarm runs — child
//! module of `swarm` (split for the 200-line cap).
use super::*;
use crate::broker::tick::{text_streaming_enabled, TextGate};

/// Map one HiveEvent to chat-facing events. Raw `Hive` forwarding happens at
/// the call site; this returns only the human-readable translations.
///
/// Agents are named by their task's *specialty*, not its title: the roster
/// lights a row by matching the active name against a roster name, so a title
/// could never match. Titles still name the *work*, but they reach the app on
/// `HivePlan` — this translation is not given them at all, so naming an agent
/// after one is impossible here rather than merely discouraged.
///
/// `gates` holds one [`TextGate`] per agent, keyed by the `AgentId`'s numeric
/// id (`agent.0`) rather than its display name, and `now_ms` is the run's
/// elapsed clock: `translate` sees one event at a time and has no clock of
/// its own, so mid-reply pacing has to be threaded in from the drain loop the
/// same way `agent_task` is.
///
/// Keying by id — not by `agent_name(...)`'s specialty string — matters
/// because specialty is LLM-authored free text with no uniqueness
/// constraint, and the scheduler runs up to `CONCURRENCY` tasks in
/// parallel: two concurrently-running tasks CAN share a specialty. Keying
/// the gate by name would collapse both agents onto one `TextGate`, so
/// their fragments would interleave into a single buffer and be emitted as
/// one `Delta` — genuine cross-agent text concatenation, not merely shared
/// attribution. Keying by `agent.0` gives every real agent its own buffer,
/// so no two agents' text can ever land in one payload.
///
/// This does NOT fix same-specialty agents being *displayed* as one: the
/// emitted `Delta`'s `agent` field is still `agent_name(...)`, so two
/// concurrent agents sharing a specialty still emit Deltas under the same
/// name and the app still merges them into one card. That is the
/// pre-existing agent-naming conflation — `Activity` and `StatsTick` already
/// name agents by specialty the same way — not something this gate solves
/// or was ever meant to.
pub(super) fn translate(
    ev: &HiveEvent,
    specialties: &HashMap<TaskId, String>,
    agent_task: &mut HashMap<u64, TaskId>,
    gates: &mut HashMap<u64, TextGate>,
    now_ms: u64,
) -> Vec<PluginEvent> {
    let specialist_of = |t: &TaskId| {
        specialties
            .get(t)
            .cloned()
            .unwrap_or_else(|| format!("specialist-{}", t.0))
    };
    let agent_name = |a: &AgentId, agent_task: &HashMap<u64, TaskId>| {
        agent_task
            .get(&a.0)
            .map(specialist_of)
            .unwrap_or_else(|| format!("agent-{}", a.0))
    };
    match ev {
        HiveEvent::AgentSpawned { agent, task } => {
            agent_task.insert(agent.0, *task);
            vec![PluginEvent::Activity {
                agent: specialist_of(task),
                state: "thinking".into(),
                from: "hive".into(),
            }]
        }
        HiveEvent::TaskStateChanged { task, state } => match state {
            TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                vec![PluginEvent::Activity {
                    agent: specialist_of(task),
                    state: "idle".into(),
                    from: String::new(),
                }]
            }
            _ => vec![],
        },
        HiveEvent::TokenDelta { agent, output, .. } => vec![PluginEvent::StatsTick {
            agent: agent_name(agent, agent_task),
            tokens: u64::from(*output),
        }],
        HiveEvent::CostDelta { .. } => vec![],
        HiveEvent::OutputDelta { agent, text } => {
            if !text_streaming_enabled() {
                return vec![];
            }
            let name = agent_name(agent, agent_task);
            let gate = gates.entry(agent.0).or_insert_with(TextGate::new);
            match gate.push(text, now_ms) {
                Some(payload) => vec![PluginEvent::Delta {
                    agent: name,
                    text: payload,
                }],
                None => vec![],
            }
        }
        HiveEvent::OutputChunk { agent, text } => {
            vec![msg(agent_name(agent, agent_task).as_str(), text.clone())]
        }
        // A task failure is chat-visible content, not a connection loss: the
        // app's chat pane treats `PluginEvent::Error` as the broker connection
        // dropping (sets connected=false and discards the text), so surface
        // this as a normal message from the failing agent/task instead.
        HiveEvent::Failed { agent, error } => {
            vec![msg(
                agent_name(agent, agent_task).as_str(),
                format!("\u{2717} failed: {error}"),
            )]
        }
    }
}
