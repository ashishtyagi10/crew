//! HiveEvent → chat-facing PluginEvent translation for swarm runs — child
//! module of `swarm` (split for the 200-line cap).
use super::*;
use crate::broker::tick::{text_streaming_enabled, TextGate};

/// Chars of tool output a result card carries. Generous because the card is
/// FOLDED to one line until clicked — a long result costs scroll the reader
/// opted into, not scroll imposed on them — and bounded because the transcript
/// is held in memory and a `curl` of a large page has no ceiling of its own.
pub(super) const RESULT_CLIP: usize = 4_000;

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
        // Tool use is shown in the transcript with the SAME `[tool]` line the
        // relay uses (`toolline::call_line` — subject-first, so a reader sees
        // which file or which command, not 200 characters of JSON). Two
        // engines rendering the same action two ways would read as two
        // different features.
        HiveEvent::ToolCall { agent, label, args } => {
            let name = agent_name(agent, agent_task);
            vec![
                msg(
                    name.as_str(),
                    format!(
                        "[tool] {}",
                        crate::broker::toolline::call_line(label, args, 200)
                    ),
                ),
                // …and say the agent is ON A TOOL, not thinking. The header
                // counts up either way; only this says whose wait it is.
                PluginEvent::Activity {
                    agent: name,
                    state: format!("tool {label}"),
                    from: "hive".into(),
                },
            ]
        }
        // Every result is shown, outcome and duration first, output beneath.
        //
        // Successful results used to be dropped, on the reasoning that pasting
        // raw tool output into the transcript would bury the agent's own
        // answer under a page of JSON. That was sound while a tool card was an
        // ordinary agent card that always rendered in full — and it stopped
        // being sound once tool cards FOLD (`chatfold`). Folded, this is one
        // line saying what happened and how long it took; clicked open, it is
        // what the API actually returned. Dropping it left no way to see that
        // at all — only the agent's paraphrase, which is the one thing a
        // person checking an integration cannot take on trust.
        HiveEvent::ToolResult {
            agent,
            label,
            ok,
            text,
            ms,
        } => {
            let head = crate::broker::toolline::result_line(label, *ok, *ms);
            // `toolclip`, NOT `route::clip`: the latter flattens whitespace,
            // which would fold every line of output onto the card's first
            // line — the one line the fold shows — so a folded result would
            // BE the whole result, and clicking it open would reveal nothing.
            let body = crate::broker::toolclip::clip_result(text.trim_end(), RESULT_CLIP);
            let card = if body.is_empty() {
                format!("[tool] {head}")
            } else {
                format!("[tool] {head}\n{body}")
            };
            let name = agent_name(agent, agent_task);
            vec![
                msg(name.as_str(), card),
                // Back to thinking — bare `tool` clears the label without
                // starting a new hop (see the app's `absorb_activity`).
                PluginEvent::Activity {
                    agent: name,
                    state: "tool".into(),
                    from: String::new(),
                },
            ]
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
