//! Routing `crew ask` to the canvas that can answer it.
//!
//! The socket used to belong to the launch canvas, which made every pane in every other window
//! unaddressable: `crew panes` listed one window and `crew ask` could only reach it. A window is
//! not a process, so the endpoint belongs to the process's owner — here — and every request is
//! handed to the canvas that owns its answer.
//!
//! Addressing keeps the spelling it had. The first window's panes are `p0`, `p1` exactly as
//! before; a second window's are `w1p0`. A pane addressed by NAME searches every window, because
//! a name is unique to the pane rather than to the window it happens to be in.
use crate::ipc_types::{CastMode, Reply, Request};

use super::Crew;
/// One broadcast in flight across the canvases: each answers into a channel of its own, and the
/// client hears one merged reply when they all have.
pub(super) struct Merge {
    reply: std::sync::mpsc::Sender<Reply>,
    mode: CastMode,
    /// One receiver per canvas that had eligible panes.
    parts: Vec<std::sync::mpsc::Receiver<Reply>>,
    answers: Vec<crate::ipc_types::CastAnswer>,
}

impl Crew {
    /// Drain the ask socket and answer from whichever canvas can. Every canvas still advances
    /// its own in-flight asks on its own tick; this is only the routing.
    pub(super) fn pump_ipc(&mut self, now_ms: u64) {
        let incoming: Vec<_> = match &self.ipc {
            Some(h) => h.rx.try_iter().collect(),
            None => Vec::new(),
        };
        for inc in incoming {
            self.route_request(inc.req, inc.reply, now_ms);
        }
        self.settle_casts();
    }

    /// Send one request to the canvas that owns its answer.
    fn route_request(&mut self, req: Request, reply: std::sync::mpsc::Sender<Reply>, now_ms: u64) {
        match req {
            // The roster is every window's panes, in window order, each with the id that
            // reaches it. A pane you cannot see listed is a pane nobody will ask.
            Request::Panes { .. } => {
                let panes = self
                    .canvases
                    .iter()
                    .enumerate()
                    .flat_map(|(w, c)| crate::panes_roster::roster_in(w, &c.panes, &c.procnames))
                    .collect();
                let _ = reply.send(Reply::Roster { panes });
            }
            Request::Broadcast {
                from,
                question,
                id,
                mode,
                ..
            } => self.fan_broadcast(from, question, id, mode, reply, now_ms),
            Request::Ask { .. } => {
                let Some(i) = self.canvas_for(&req) else {
                    let _ = reply.send(Reply::NoAnswer {
                        reason: crate::ipc_types::NoAnswer::Unreachable,
                        partial: None,
                    });
                    return;
                };
                self.canvases[i].service_request(req, reply, now_ms);
            }
            // Daemon ops belong to the resident's endpoint, not this one.
            _ => {}
        }
    }

    /// Which canvas an `Ask` is for: the window its address names, else the first that can
    /// resolve the address at all. A bare name searches every window, so `crew ask schema` keeps
    /// working when the pane called `schema` is in the second one.
    fn canvas_for(&self, req: &Request) -> Option<usize> {
        let Request::Ask { to, .. } = req else {
            return None;
        };
        let (pane, _) = crate::askroute::split_instance(to);
        let (window, rest) = crate::askroute::split_window(pane);
        if let Some(w) = window {
            return (w < self.canvases.len()
                && crate::askroute::resolve(&self.canvases[w].panes, rest).is_some())
            .then_some(w);
        }
        self.canvases
            .iter()
            .position(|c| crate::askroute::resolve(&c.panes, rest).is_some())
    }

    /// Fan one broadcast across every canvas that has panes, and remember to merge them.
    fn fan_broadcast(
        &mut self,
        from: String,
        question: String,
        id: String,
        mode: CastMode,
        reply: std::sync::mpsc::Sender<Reply>,
        now_ms: u64,
    ) {
        let mut parts = Vec::new();
        for c in self.canvases.iter_mut().filter(|c| !c.panes.is_empty()) {
            let (tx, rx) = std::sync::mpsc::channel();
            c.service_broadcast(from.clone(), question.clone(), id.clone(), mode, tx, now_ms);
            parts.push(rx);
        }
        if parts.is_empty() {
            let _ = reply.send(Reply::Cast {
                answers: Vec::new(),
            });
            return;
        }
        self.casts.push(Merge {
            reply,
            mode,
            parts,
            answers: Vec::new(),
        });
    }

    /// Collect whatever the canvases have answered, and reply when a broadcast is done.
    fn settle_casts(&mut self) {
        let mut done = Vec::new();
        for (i, m) in self.casts.iter_mut().enumerate() {
            m.parts.retain(|rx| match rx.try_recv() {
                Ok(Reply::Cast { answers }) => {
                    m.answers.extend(answers);
                    false
                }
                // A canvas that closed mid-broadcast answers nothing; dropping its receiver
                // must end the wait rather than hanging the client forever.
                Err(std::sync::mpsc::TryRecvError::Disconnected) => false,
                _ => true,
            });
            // `Any` settles on the first REAL answer, exactly as one canvas does — the others
            // were asked anyway, and their answers are dropped.
            let any_done =
                matches!(m.mode, CastMode::Any) && m.answers.iter().any(|a| a.text.is_some());
            if m.parts.is_empty() || any_done {
                done.push(i);
            }
        }
        for i in done.into_iter().rev() {
            let m = self.casts.remove(i);
            let answers = match m.mode {
                CastMode::Any => m
                    .answers
                    .into_iter()
                    .find(|a| a.text.is_some())
                    .into_iter()
                    .collect(),
                CastMode::All => m.answers,
            };
            let _ = m.reply.send(Reply::Cast { answers });
        }
    }
}

#[cfg(test)]
#[path = "ask_tests.rs"]
mod tests;
