//! Broadcast inter-pane `ask` (`crew ask --all` / `--any`): the v2 resolver
//! widens ONE address to a SET of panes, fans the question into each, and
//! aggregates the per-pane verdicts into a single reply. The liveness engine
//! (askwait) and cooperative sentinel (askroute) are reused unchanged — only
//! the resolver (a set, not one pane) and the settle rule are new. The
//! envelope is transport-agnostic, so v3 federation swaps the local pane set
//! for a federated one without touching this file
//! (docs/vision/sentinel-network.md).
use std::io::Write;
use std::sync::mpsc::Sender;

use crate::app::CrewApp;
use crate::askwait::{Obs, PendingAsk, Step};
use crate::ipc_types::{CastAnswer, CastMode, NoAnswer, Reply};
use crate::pane::{Pane, PaneContent};

/// Absolute backstop per pane — the same ceiling a targeted ask uses.
const CEILING_MS: u64 = 180_000;

/// One in-flight broadcast: the per-pane asks still running, the answers
/// already collected, the settle mode, and the channel the aggregate goes back
/// on when it resolves.
pub(crate) struct Casting {
    pub reply: Sender<Reply>,
    pub mode: CastMode,
    pub pending: Vec<CastTarget>,
    pub collected: Vec<CastAnswer>,
}

/// A single pane within a broadcast: its liveness state plus the identity it is
/// reported under in the aggregate.
pub(crate) struct CastTarget {
    pub ask: PendingAsk,
    pub pane: String,
    pub label: Option<String>,
}

impl CrewApp {
    /// Service a `Request::Broadcast`: fan the question into every eligible pane
    /// (terminal panes other than the asker), tapping each and registering an
    /// aggregate. Returns true if anything was injected (screen changed).
    pub(crate) fn service_broadcast(
        &mut self,
        from: String,
        question: String,
        id: String,
        mode: CastMode,
        reply: Sender<Reply>,
        now_ms: u64,
    ) -> bool {
        let targets: Vec<usize> = self
            .panes
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.content, PaneContent::Terminal(_)))
            .filter(|(_, p)| p.name.as_deref().or(p.label.as_deref()) != Some(from.as_str()))
            .map(|(i, _)| i)
            .collect();
        let mut pending = Vec::new();
        for idx in targets {
            let pane_id = format!("p{idx}");
            let label = self.panes[idx]
                .name
                .clone()
                .or_else(|| self.panes[idx].label.clone());
            let sub_id = format!("{id}.{idx}");
            let PaneContent::Terminal(t) = &mut self.panes[idx].content else {
                continue;
            };
            t.pty.start_capture();
            let wrapped = crate::askroute::wrap(&from, &sub_id, &question);
            let _ = t
                .input
                .write_all(wrapped.as_bytes())
                .and_then(|_| t.input.write_all(b"\n"))
                .and_then(|_| t.input.flush());
            pending.push(CastTarget {
                ask: PendingAsk::new(sub_id, idx, now_ms),
                pane: pane_id,
                label,
            });
        }
        if pending.is_empty() {
            let _ = reply.send(Reply::Cast { answers: vec![] });
            return false;
        }
        self.castings.push(Casting {
            reply,
            mode,
            pending,
            collected: Vec::new(),
        });
        true
    }

    /// Advance every in-flight broadcast; reply and drop each as it settles.
    pub(crate) fn tick_castings(&mut self, now_ms: u64) {
        let mut done: Vec<usize> = Vec::new();
        for (ci, cast) in self.castings.iter_mut().enumerate() {
            let mut still: Vec<CastTarget> = Vec::new();
            for mut ct in std::mem::take(&mut cast.pending) {
                let new_output = match self.panes.get_mut(ct.ask.target) {
                    Some(p) => match &mut p.content {
                        PaneContent::Terminal(t) => t.pty.take_capture(),
                        _ => String::new(),
                    },
                    None => String::new(),
                };
                let over = now_ms.saturating_sub(ct.ask.asked_ms) > CEILING_MS;
                let step = ct.ask.observe(Obs {
                    new_output: &new_output,
                    idle_transition: over,
                    now_ms,
                });
                if matches!(step, Step::Wait) {
                    still.push(ct);
                } else {
                    stop_capture(&mut self.panes, ct.ask.target);
                    cast.collected.push(answer_of(&ct, step));
                }
            }
            cast.pending = still;
            if let Some(answers) = settle(cast.mode, &cast.collected, cast.pending.is_empty()) {
                for ct in std::mem::take(&mut cast.pending) {
                    stop_capture(&mut self.panes, ct.ask.target);
                }
                let _ = cast.reply.send(Reply::Cast { answers });
                done.push(ci);
            }
        }
        for i in done.into_iter().rev() {
            self.castings.remove(i);
        }
    }
}

/// The settle decision, pure over the collected answers and whether any target
/// is still pending. `Some(answers)` means reply now with those:
/// `--any` fires the instant a real answer lands (only the winners); either
/// mode fires with everything once no target is still pending.
fn settle(
    mode: CastMode,
    collected: &[CastAnswer],
    pending_empty: bool,
) -> Option<Vec<CastAnswer>> {
    let answered: Vec<CastAnswer> = collected
        .iter()
        .filter(|a| a.text.is_some())
        .cloned()
        .collect();
    match mode {
        CastMode::Any if !answered.is_empty() => Some(answered),
        _ if pending_empty => Some(collected.to_vec()),
        _ => None,
    }
}

/// Stop tapping a target pane's output (best-effort; pane may have vanished).
fn stop_capture(panes: &mut [Pane], target: usize) {
    if let Some(PaneContent::Terminal(t)) = panes.get_mut(target).map(|p| &mut p.content) {
        t.pty.stop_capture();
    }
}

/// Turn a resolved liveness `Step` into the pane's aggregate entry.
fn answer_of(ct: &CastTarget, step: Step) -> CastAnswer {
    let (text, no_answer) = match step {
        Step::Answered(t) => (Some(t), None),
        Step::Stalled(_) => (None, Some(NoAnswer::Stalled)),
        Step::IdleNoEngage => (None, Some(NoAnswer::IdleNoEngage)),
        Step::Wait => (None, None),
    };
    CastAnswer {
        pane: ct.pane.clone(),
        label: ct.label.clone(),
        text,
        no_answer,
    }
}

#[cfg(test)]
#[path = "askcast_tests.rs"]
mod tests;
