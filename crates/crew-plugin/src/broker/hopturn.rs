//! A relay hop on the harness: an agent's whole turn — its model calls and
//! every tool round between them — run by `crew_hive::harness`, the loop
//! every swarm task runs on. The relay had written the turn a second time,
//! and the two had drifted (`docs/superpowers/goals/2026-10-24-one-harness.md`);
//! it now has none of its own.
//!
//! The agent is the model the turn calls (`model`). An API agent's provider
//! gets native tool-use when it speaks it: each tool's JSON Schema on the
//! wire, the arguments checked by the provider, a call whose arguments were
//! not JSON refused with the reason. A CLI agent is a model with no tool-use,
//! so the `@tool` lines it prints are the text loop's to run, and the tools
//! it runs itself stay its own. Both get the last word when their rounds run
//! out.
//!
//! What is the relay's own stays the broker's and is handed in: the frame,
//! its tools hint in place (`TurnSpec::hinted`) and its recalled turns the
//! first thing to go when the model refuses the length (`TurnSpec::shed`);
//! the pane, as [`Pane`], the hop's stream behind the harness's events; what
//! the user types mid-turn (`steer`); and the advisor, as [`Advised`]. Hop
//! routing is untouched: the answer comes back to `Broker::run` as the hop's
//! reply.
use std::sync::{Arc, Mutex, MutexGuard};

use crew_hive::harness::{self, Advice, Ended, Events, TurnContext, TurnSpec};
use crew_hive::steers::Steers;
use crew_hive::tools::{budget::ToolBudget, todowire, ToolCall};
use crew_hive::{AgentId, HiveEvent, ModelTier, Provider};

use super::adapter::{Adapter, HopStream, Usage};
use super::hop::{back, Hop, HopKind, RunStats};
use super::toolcall::ToolRunner;
use super::toolround::advise::Advisor;
use super::{Broker, Envelope};

#[path = "hopmodel.rs"]
mod model;

/// Output token ceiling per model call of a CLI agent's turn: unread, since
/// a CLI picks its own (`ApiAdapter`'s is its engine's).
const CLI_MAX_TOKENS: u32 = super::apiadapter::MAX_TOKENS;

/// Chars of a result the tool line keeps for its click-open view.
const RESULT_KEEP: usize = 2_000;

/// What one turn ran up, kept apart while it runs and added to the thread's
/// stats when it ends.
#[derive(Default)]
struct Tally {
    /// What the turn's model calls and its advisor cost.
    stats: RunStats,
    /// The latest call's usage, an earlier fallback kept (`Usage::latest`).
    usage: Usage,
    /// Tool calls made, for the note under a turn stopped or failed after one.
    calls: u32,
    /// Reply chars streamed over every call of the turn: the hop's running
    /// token estimate, which only grows, so the tick gate never swallows a
    /// short follow-up after a long first reply.
    chars: u64,
    /// A checklist write between its call and its result: it goes out as the
    /// plan once accepted, and not at all when refused.
    todo: Option<HiveEvent>,
    /// A CLI agent's own words for why its last call failed, for the error
    /// card as its dial would have shown them.
    said: Option<String>,
    /// What the latest reply that asked for tools wrote above its calls: an
    /// answer a failure or a stop cut short keeps it, rather than ending on
    /// a note alone.
    wrote: String,
}

impl Tally {
    /// One call's reply landed: its usage, and what it wrote with any calls.
    fn landed(&mut self, sent: usize, c: &crew_hive::Completion, used: Usage) {
        self.stats.add((sent + c.text.len()) / 4, &used);
        self.usage.latest(used);
        match crew_hive::tools::split_tool_calls(&c.text) {
            Some((above, _)) => self.wrote = above,
            None if !c.calls.is_empty() => self.wrote = c.text.clone(),
            None => {}
        }
    }
}

type Shared = Arc<Mutex<Tally>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The harness's events, into the hop's stream: reply text and reasoning as
/// they arrive, each of crew's tool calls and its result on the tool lane the
/// pane draws Claude Code's on, so it cannot tell the two apart, and a
/// checklist write as the plan, never as a tool line (`hoptool`). The answer,
/// the failure and the bills are not forwarded: the hop's reply, its error
/// hop and the model's usage say those.
struct Pane {
    stream: HopStream,
    tally: Shared,
}

impl Events for Pane {
    fn publish(&self, ev: HiveEvent) {
        match ev {
            HiveEvent::OutputDelta { text, .. } => {
                (self.stream.on_text)(&text);
                let chars = {
                    let mut t = lock(&self.tally);
                    t.chars += text.chars().count() as u64;
                    t.chars
                };
                (self.stream.on_tokens)(chars / 4);
            }
            HiveEvent::ThoughtDelta { text, .. } if !text.is_empty() => {
                (self.stream.on_thought)(&text);
            }
            HiveEvent::ToolCall { ref label, .. } if todowire::is_list(label) => {
                let mut t = lock(&self.tally);
                t.calls += 1;
                t.todo = Some(ev);
            }
            // A refused write changed nothing: the old list stands.
            HiveEvent::ToolResult { ref label, ok, .. } if todowire::is_list(label) => {
                let held = lock(&self.tally).todo.take();
                if let (true, Some(call)) = (ok, held) {
                    (self.stream.on_tool)(call);
                }
            }
            HiveEvent::ToolCall { .. } => {
                lock(&self.tally).calls += 1;
                (self.stream.on_tool)(ev);
            }
            HiveEvent::ToolResult {
                agent,
                label,
                ok,
                text,
                ms,
            } => {
                lock(&self.tally).stats.approx_tokens += text.len() / 4;
                (self.stream.on_tool)(HiveEvent::ToolResult {
                    agent,
                    label,
                    ok,
                    text: kept(&text),
                    ms,
                });
            }
            _ => {}
        }
    }
}

/// The head of a result the tool line keeps, marked when there was more.
fn kept(text: &str) -> String {
    let mut kept: String = text.trim_end().chars().take(RESULT_KEEP).collect();
    if text.trim_end().chars().count() > RESULT_KEEP {
        kept.push_str("\n\u{2026}");
    }
    kept
}

/// `answer` with its routing line last again. The harness writes its own
/// notes under a reply — the checklist's unfinished steps, the spent budget —
/// and the reply's own last line was its `@done` or `@next`, the only line
/// routing reads (`route::parse_routing`): under a note, the `@done` stayed
/// in the answer as text and an `@next` hand-off was lost. The notes stay,
/// above it.
fn routed_last(answer: &str) -> String {
    let lines: Vec<&str> = answer.trim().lines().collect();
    let note = |l: &str| {
        let l = l.trim();
        l.is_empty() || l.starts_with("checklist: ") || l.starts_with("[tool budget spent")
    };
    let Some(at) = lines.iter().rposition(|l| !note(l)) else {
        return answer.trim().to_string();
    };
    let below = &lines[at + 1..];
    if below.is_empty() || !super::route::has_directive(lines[at]) {
        return answer.trim().to_string();
    }
    let above = lines[..at].join("\n");
    let notes = below.join("\n");
    format!(
        "{}\n\n{}\n{}",
        above.trim_end(),
        notes.trim(),
        lines[at].trim()
    )
    .trim()
    .to_string()
}

/// The relay's advisor, read each round (`toolround::advise`): a second model
/// on a failed check, its cost added to the turn's.
struct Advised {
    advisor: Mutex<Advisor>,
    runner: Arc<dyn ToolRunner>,
    env: Envelope,
    tally: Shared,
}

impl Advice for Advised {
    fn read(&self, calls: &[ToolCall], ran: &[(bool, String)]) -> String {
        let mut spent = RunStats::default();
        let said = lock(&self.advisor).read(
            self.runner.as_ref(),
            &self.env,
            calls,
            ran,
            &mut spent,
            false,
        );
        lock(&self.tally).stats.merge(&spent);
        said
    }
}

impl Broker {
    /// One hop of `agent` with this broker's tools, run whole on the
    /// harness: its reply and usage for routing, or `None` when the hop ended
    /// on an error card (sunk here), as a dial that failed. `prompt` is the
    /// frame without its tools hint, `hinted` with it, and `ask` the message
    /// the tools are chosen for.
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    pub(super) fn harnessed(
        &self,
        agent: &Arc<dyn Adapter>,
        env: &Envelope,
        ask: &str,
        prompt: String,
        hinted: String,
        stats: &mut RunStats,
        stream: &HopStream,
        sink: &mut dyn FnMut(Hop),
    ) -> Option<(String, Usage)> {
        let runner = self.tools.clone()?;
        let tally: Shared = Arc::default();
        let engine = agent.engine();
        // A CLI's calls block on a thread of their own (`model::Spoken`);
        // this runtime only drives the turn around them.
        let own;
        let rt = match &engine {
            Some(e) => e.rt,
            None => match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => {
                    own = rt;
                    &own
                }
                Err(e) => {
                    sink(back(
                        env,
                        HopKind::Error,
                        format!("no runtime for the turn: {e}"),
                    ));
                    return None;
                }
            },
        };
        let (provider, model, system, max_tokens): (Arc<dyn Provider>, _, _, _) = match &engine {
            Some(e) => (
                Arc::new(model::Bounded {
                    inner: Arc::clone(e.provider),
                    model: e.model.to_string(),
                    timeout: self.timeout,
                    tally: Arc::clone(&tally),
                }),
                e.model.to_string(),
                e.system.map(str::to_string),
                e.max_tokens,
            ),
            None => (
                Arc::new(model::Spoken {
                    agent: Arc::clone(agent),
                    timeout: self.timeout,
                    lane: Arc::clone(&stream.on_tool),
                    tally: Arc::clone(&tally),
                }),
                agent.model().to_string(),
                None,
                CLI_MAX_TOKENS,
            ),
        };
        let advised = Advised {
            advisor: Mutex::default(),
            runner: Arc::clone(&runner),
            env: env.clone(),
            tally: Arc::clone(&tally),
        };
        let ctx = TurnContext {
            agent: AgentId::minted(&env.to),
            events: Arc::new(Pane {
                stream: stream.clone(),
                tally: Arc::clone(&tally),
            }),
            budget: ToolBudget::turn(self.tool_rounds),
            cancel: self.stop_flag(),
            advice: Some(Arc::new(advised)),
        };
        let spec = TurnSpec {
            ask: ask.to_string(),
            prompt,
            hinted: Some(hinted),
            // The recalled turns are the frame's to lose first.
            shed: Some(super::recall::cut),
            // How to finish is the frame's HOW TO REPLY, pointed at, never
            // restated: alone it is `@done`, with peers `@next` or `@done`.
            ending: Some("end it as HOW TO REPLY says".into()),
            system,
            model,
            // Billing's fallback only, and the relay bills from the model's usage.
            tier: ModelTier::Standard,
            max_tokens,
        };
        let steers = Steers::new(super::steer::take).with_tail(super::steer::TAIL);
        let ended = rt.block_on(harness::run(
            ctx,
            spec,
            provider,
            Some(runner),
            Some(steers),
        ));
        (stream.on_thought)(""); // the hop is over: flush the gate's tail
        let mut tally = std::mem::take(&mut *lock(&tally));
        stats.merge(&tally.stats);
        let (made, wrote) = (tally.calls, std::mem::take(&mut tally.wrote));
        let usage = std::mem::take(&mut tally.usage);
        let error = |why: String| back(env, HopKind::Error, why);
        match ended {
            Ended::Answered(text) if !text.trim().is_empty() => Some((routed_last(&text), usage)),
            Ended::Answered(_) if made == 0 => {
                sink(error("empty reply".into()));
                None
            }
            Ended::Answered(_) => {
                sink(error("empty reply after tool call".into()));
                Some((
                    super::toolround::failed_answer(&wrote, "an empty reply"),
                    usage,
                ))
            }
            Ended::Failed { why, .. } => {
                let why = tally.said.take().unwrap_or(why);
                if made == 0 {
                    sink(error(why));
                    return None;
                }
                let answer = super::toolround::failed_answer(&wrote, &why);
                sink(error(why));
                Some((answer, usage))
            }
            Ended::Stopped if made == 0 => {
                sink(error(super::cancelscope::STOPPED.into()));
                None
            }
            Ended::Stopped => Some((super::toolround::stopped_answer(&wrote, made), usage)),
        }
    }

    /// The stop a turn on the harness honours: this broker's `/stop`, else
    /// the running task's (`cancelscope`), else one nobody will set.
    fn stop_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.cancel
            .clone()
            .or_else(super::cancelscope::current)
            .unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "hopturn_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "hopfull_tests.rs"]
mod full_tests;
