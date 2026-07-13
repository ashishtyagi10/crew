//! HiveEvent → chat-facing PluginEvent translation for swarm runs — child
//! module of `swarm` (split for the 200-line cap).
use super::*;

/// Map one HiveEvent to chat-facing events. Raw `Hive` forwarding happens at
/// the call site; this returns only the human-readable translations.
pub(super) fn translate(
    ev: &HiveEvent,
    titles: &HashMap<TaskId, String>,
    agent_task: &mut HashMap<u64, TaskId>,
) -> Vec<PluginEvent> {
    let title_of = |t: &TaskId| {
        titles
            .get(t)
            .cloned()
            .unwrap_or_else(|| format!("task-{}", t.0))
    };
    let agent_name = |a: &AgentId, agent_task: &HashMap<u64, TaskId>| {
        agent_task
            .get(&a.0)
            .map(title_of)
            .unwrap_or_else(|| format!("agent-{}", a.0))
    };
    match ev {
        HiveEvent::AgentSpawned { agent, task } => {
            agent_task.insert(agent.0, *task);
            vec![PluginEvent::Activity {
                agent: title_of(task),
                state: "thinking".into(),
                from: "hive".into(),
            }]
        }
        HiveEvent::TaskStateChanged { task, state } => match state {
            TaskState::Done | TaskState::Failed | TaskState::Cancelled => {
                vec![PluginEvent::Activity {
                    agent: title_of(task),
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
