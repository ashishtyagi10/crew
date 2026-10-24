//! A relay hop on the harness: an API agent's whole turn — its model calls
//! and every tool round between them — run by `crew_hive::harness`, the loop
//! every swarm task runs on, instead of the relay's own (`toolcall::run_tools`).
//!
//! WHY: the relay had written the turn a second time, and the two had drifted
//! (`docs/superpowers/goals/2026-10-24-one-harness.md`). On the harness an API
//! agent gets what only the swarm had: native tool-use when its provider
//! speaks it (each tool's JSON Schema on the wire, the arguments checked by
//! the provider, not hand-written on a reply's last line), the refusal of a
//! call whose arguments were not JSON, and the last word when its rounds run
//! out. CLI agents keep `run_tools` until they are wrapped as a model too.
//!
//! What is the relay's own stays the broker's and is handed in: the frame,
//! its tools hint in place (`TurnSpec::hinted`); the pane, as [`Pane`], the
//! hop's stream behind the harness's events; the call bound, the cut-off
//! continuation and the usage, as [`Bounded`], a provider around the agent's
//! own; what the user types mid-turn (`steer`); and the advisor, as
//! [`Advised`]. Hop routing is untouched: the answer comes back to
//! `Broker::run` as the hop's reply, as `run_tools`' did.
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crew_hive::harness::{self, Advice, Ended, Events, TurnContext, TurnSpec};
use crew_hive::steers::Steers;
use crew_hive::tools::{budget::ToolBudget, todowire, ToolCall};
use crew_hive::{
    AgentId, ChunkFn, Completion, CompletionRequest, HiveEvent, ModelTier, Provider, ProviderError,
    Turn,
};

use super::adapter::{Engine, HopStream, Usage};
use super::hop::{back, Hop, HopKind, RunStats};
use super::toolbatch::Ran;
use super::toolcall::ToolRunner;
use super::toolround::advise::Advisor;
use super::{Broker, Envelope};

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
    /// plan once accepted, and not at all when refused (`planout`).
    todo: Option<HiveEvent>,
}

type Shared = Arc<Mutex<Tally>>;

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// The harness's events, into the hop's stream: reply text and reasoning as
/// they arrive, each tool call and result on the tool lane the pane draws
/// Claude Code's on (`toolhive`), a checklist write as the plan (`planout`).
/// The answer, the failure and the bills are not forwarded: the hop's reply,
/// its error hop and [`Bounded`]'s usage say those.
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
                    text: super::toolhive::kept(&text),
                    ms,
                });
            }
            _ => {}
        }
    }
}

/// The agent's provider as a relay hop calls it, each call as the agent's
/// own dial made it: bounded by the broker's timeout, continued once when
/// the token ceiling cut it (`cutoff`), and its usage kept for the hop.
struct Bounded {
    inner: Arc<dyn Provider>,
    model: String,
    timeout: Duration,
    tally: Shared,
}

type Call = Pin<Box<dyn Future<Output = Result<Completion, ProviderError>> + Send>>;

impl Bounded {
    fn call(&self, req: CompletionRequest, on_chunk: Option<ChunkFn>) -> Call {
        let (inner, tally) = (Arc::clone(&self.inner), Arc::clone(&self.tally));
        let (model, timeout) = (self.model.clone(), self.timeout);
        let sent = sent_chars(&req);
        Box::pin(async move {
            let whole = super::cutoff::whole(inner, req, on_chunk, timeout);
            let c = match tokio::time::timeout(timeout, whole).await {
                Ok(got) => got?,
                Err(_) => {
                    return Err(ProviderError::Http(format!(
                    "{model}: api call timed out after {timeout:?} (raise CREW_BROKER_TIMEOUT_MS?)"
                )))
                }
            };
            let used = Usage::of(&model, &c);
            let mut t = lock(&tally);
            t.stats.add((sent + c.text.len()) / 4, &used);
            t.usage.latest(used);
            Ok(c)
        })
    }
}

impl Provider for Bounded {
    fn supports_tools(&self) -> bool {
        self.inner.supports_tools()
    }

    fn complete(&self, req: CompletionRequest) -> Call {
        self.call(req, None)
    }

    fn warm(&self) {
        self.inner.warm();
    }

    fn complete_streaming(&self, req: CompletionRequest, on_chunk: ChunkFn) -> Call {
        self.call(req, Some(on_chunk))
    }
}

/// Chars a request sends, its turns included, for the chars/4 estimate.
fn sent_chars(req: &CompletionRequest) -> usize {
    let turns: usize = req
        .turns
        .iter()
        .map(|t| match t {
            Turn::User(text) => text.len(),
            Turn::Assistant { text, calls } => {
                text.len()
                    + calls
                        .iter()
                        .map(|c| c.input.to_string().len())
                        .sum::<usize>()
            }
            Turn::ToolResults(results) => results.iter().map(|r| r.content.len()).sum(),
        })
        .sum();
    req.prompt.len() + turns
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
        let ran: Vec<Ran> = ran
            .iter()
            .map(|(ok, text)| Ran {
                ok: *ok,
                text: text.clone(),
                ms: 0,
            })
            .collect();
        let mut spent = RunStats::default();
        let said = lock(&self.advisor).read(
            self.runner.as_ref(),
            &self.env,
            calls,
            &ran,
            &mut spent,
            false,
        );
        lock(&self.tally).stats.merge(&spent);
        said
    }
}

impl Broker {
    /// One hop of the API agent `engine` with this broker's tools, run whole
    /// on the harness: its reply and usage for routing, or `None` when the
    /// hop ended on an error card (sunk here), as a dial that failed before
    /// `run_tools` ended it. `prompt` is the frame without its tools hint,
    /// `hinted` with it, and `ask` the message the tools are chosen for.
    #[allow(clippy::too_many_arguments)] // engine-loop plumbing, one call site
    pub(super) fn api_turn(
        &self,
        engine: Engine<'_>,
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
            system: engine.system.map(str::to_string),
            model: engine.model.to_string(),
            // Billing's fallback only, and the relay bills from `Bounded`.
            tier: ModelTier::Standard,
            max_tokens: engine.max_tokens,
        };
        let provider = Arc::new(Bounded {
            inner: Arc::clone(engine.provider),
            model: engine.model.to_string(),
            timeout: self.timeout,
            tally: Arc::clone(&tally),
        });
        let steers = Steers::new(super::steer::take).with_tail(super::steer::TAIL);
        let turn = harness::run(ctx, spec, provider, Some(runner), Some(steers));
        let ended = engine.rt.block_on(turn);
        (stream.on_thought)(""); // the hop is over: flush the gate's tail
        let tally = std::mem::take(&mut *lock(&tally));
        stats.merge(&tally.stats);
        let (usage, made) = (tally.usage, tally.calls);
        let error = |why: String| back(env, HopKind::Error, why);
        match ended {
            Ended::Answered(text) if !text.trim().is_empty() => {
                Some((text.trim().to_string(), usage))
            }
            Ended::Answered(_) if made == 0 => {
                sink(error("empty reply".into()));
                None
            }
            Ended::Answered(_) => {
                sink(error("empty reply after tool call".into()));
                let answer = super::toolround::failed_answer("", "an empty reply");
                Some((answer, usage))
            }
            Ended::Failed { why, .. } if made == 0 => {
                sink(error(why));
                None
            }
            Ended::Failed { why, .. } => {
                let answer = super::toolround::failed_answer("", &why);
                sink(error(why));
                Some((answer, usage))
            }
            Ended::Stopped if made == 0 => {
                sink(error(super::cancelscope::STOPPED.into()));
                None
            }
            Ended::Stopped => Some((super::toolround::stopped_answer("", made), usage)),
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
#[path = "apiturn_tests.rs"]
mod tests;
