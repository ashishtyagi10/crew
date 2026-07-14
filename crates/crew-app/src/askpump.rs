//! Drives inter-pane `ask` on the winit poll tick: drains socket requests
//! (serve the roster, or resolve+inject a question and register a pending
//! ask), and advances each pending ask's liveness engine against its target
//! pane's captured output — sending a verdict back when it resolves.
//!
//! v1 delivers to TERMINAL panes only (inject the sentinel-wrapped question
//! into the pane's live session, visibly). Non-terminal targets resolve
//! `Unreachable`; swarm-agent delivery is a later phase.
use std::sync::mpsc::Sender;

use crate::app::CrewApp;
use crate::askwait::{Obs, PendingAsk, Step};
use crate::ipc_types::{NoAnswer, Reply, Request};
use crate::pane::PaneContent;

/// Absolute backstop: a single ask never lives longer than this, even if the
/// target keeps emitting unrelated output forever. Liveness normally resolves
/// far sooner.
const CEILING_MS: u64 = 180_000;

impl CrewApp {
    /// Pump inter-pane ask I/O for this tick. Returns true if anything changed
    /// the screen (an injected question, so the caller repaints).
    pub(crate) fn pump_asks(&mut self, now_ms: u64) -> bool {
        let mut changed = false;
        // Drain newly-arrived requests (non-blocking).
        let incoming: Vec<_> = match &self.ipc {
            Some(h) => h.rx.try_iter().collect(),
            None => Vec::new(),
        };
        for inc in incoming {
            changed |= self.service_request(inc.req, inc.reply, now_ms);
        }
        self.tick_pending(now_ms);
        changed
    }

    /// Handle one request: serve the roster, or resolve + inject + register.
    /// Returns true if it injected (screen changed).
    fn service_request(&mut self, req: Request, reply: Sender<Reply>, now_ms: u64) -> bool {
        let (from, to, question, id) = match req {
            Request::Panes { .. } => {
                let panes = crate::panes_roster::roster(&self.panes, &self.procnames);
                let _ = reply.send(Reply::Roster { panes });
                return false;
            }
            Request::Ask {
                from,
                to,
                question,
                id,
                ..
            } => (from, to, question, id),
        };
        let Some(idx) = crate::askroute::resolve(&self.panes, &to) else {
            let _ = reply.send(Reply::NoAnswer {
                reason: NoAnswer::Unreachable,
                partial: None,
            });
            return false;
        };
        // v1: only terminal panes carry an addressable CLI agent.
        let PaneContent::Terminal(t) = &mut self.panes[idx].content else {
            let _ = reply.send(Reply::NoAnswer {
                reason: NoAnswer::Unreachable,
                partial: None,
            });
            return false;
        };
        // Tap the target's output and inject the visible, sentinel-wrapped
        // question into its live session.
        t.pty.start_capture();
        let wrapped = crate::askroute::wrap(&from, &id, &question);
        let _ = t
            .input
            .write_all(wrapped.as_bytes())
            .and_then(|_| t.input.write_all(b"\n"))
            .and_then(|_| t.input.flush());
        self.pending_asks
            .push((PendingAsk::new(id, idx, now_ms), reply));
        true
    }

    /// Advance every pending ask; resolve and reply on a verdict.
    fn tick_pending(&mut self, now_ms: u64) {
        let mut resolved: Vec<usize> = Vec::new();
        for (i, (ask, reply)) in self.pending_asks.iter_mut().enumerate() {
            // The target's new output this tick (empty if the pane vanished or
            // isn't a terminal any more).
            let new_output = match self.panes.get_mut(ask.target) {
                Some(p) => match &mut p.content {
                    PaneContent::Terminal(t) => t.pty.take_capture(),
                    _ => String::new(),
                },
                None => String::new(),
            };
            let over_ceiling = now_ms.saturating_sub(ask.asked_ms) > CEILING_MS;
            let step = ask.observe(Obs {
                new_output: &new_output,
                idle_transition: over_ceiling, // treat the ceiling like a give-up
                now_ms,
            });
            let verdict = match step {
                Step::Wait => continue,
                Step::Answered(text) => Reply::Answered { text },
                Step::Stalled(partial) => Reply::NoAnswer {
                    reason: NoAnswer::Stalled,
                    partial,
                },
                Step::IdleNoEngage => Reply::NoAnswer {
                    reason: NoAnswer::IdleNoEngage,
                    partial: None,
                },
            };
            let _ = reply.send(verdict);
            // Stop tapping the target's output.
            if let Some(PaneContent::Terminal(t)) =
                self.panes.get_mut(ask.target).map(|p| &mut p.content)
            {
                t.pty.stop_capture();
            }
            resolved.push(i);
        }
        for i in resolved.into_iter().rev() {
            self.pending_asks.remove(i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn ask_to_unknown_address_is_unreachable() {
        let mut app = CrewApp::default(); // no panes
        let (tx, rx) = channel::<Reply>();
        app.service_request(
            Request::Ask {
                v: 1,
                from: "a".into(),
                to: "nope".into(),
                question: "q".into(),
                id: "q1".into(),
            },
            tx,
            0,
        );
        assert!(matches!(
            rx.try_recv().unwrap(),
            Reply::NoAnswer {
                reason: NoAnswer::Unreachable,
                ..
            }
        ));
        assert!(app.pending_asks.is_empty(), "no pending ask registered");
    }

    #[test]
    fn panes_request_serves_a_roster() {
        let mut app = CrewApp::default();
        let (tx, rx) = channel::<Reply>();
        app.service_request(Request::Panes { v: 1 }, tx, 0);
        assert!(matches!(rx.try_recv().unwrap(), Reply::Roster { .. }));
    }

    /// End-to-end over a REAL pty: inject a question into a terminal pane
    /// running a cooperating responder, pump the poll loop, and confirm the
    /// answer comes back — exercising inject → capture tap → liveness → verdict
    /// (and proving the echo of the injected instruction is not mis-read).
    #[test]
    fn live_terminal_pane_answers_through_the_full_pipeline() {
        use crate::layout::Rect;
        use crate::pane::{Pane, PaneContent, TermPane};
        use crew_term::{GridSize, PtyTerm};
        use std::io::Write;
        use std::time::{Duration, Instant};

        let mut pty = PtyTerm::spawn(GridSize { cols: 80, rows: 24 }, "sh").unwrap();
        // A cooperating responder: on any line that MENTIONS the marker, print
        // an answer line that BEGINS with it. (Mirrors what an instructed LLM
        // agent does; the echoed instruction line has the marker mid-line and
        // must not be mistaken for the answer.)
        {
            let mut w = pty.writer();
            w.write_all(
                b"while IFS= read -r l; do case \"$l\" in *CREW-ANS-*) \
                  printf 'CREW-ANS-qtest: 42\\n';; esac; done\n",
            )
            .unwrap();
            w.flush().unwrap();
        }
        let input = pty.writer();
        let mut app = CrewApp::default();
        app.panes.push(Pane {
            content: PaneContent::Terminal(Box::new(TermPane {
                pty,
                input,
                cmd: None,
                cmd_since: None,
            })),
            grid: GridSize { cols: 80, rows: 24 },
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });

        // Ask pane p0. Fixed id so the responder can hardcode its marker.
        let (tx, rx) = channel::<Reply>();
        app.service_request(
            Request::Ask {
                v: 1,
                from: "tester".into(),
                to: "p0".into(),
                question: "the answer please".into(),
                id: "qtest".into(),
            },
            tx,
            0,
        );
        assert_eq!(app.pending_asks.len(), 1, "ask registered");

        // Pump the loop: try_read fills the capture (as the pane-drain loop
        // does), then pump_asks advances the ask.
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut answer = None;
        let mut now = 0u64;
        while Instant::now() < deadline {
            now += 16;
            if let PaneContent::Terminal(t) = &mut app.panes[0].content {
                t.pty.try_read();
            }
            app.pump_asks(now);
            if let Ok(r) = rx.try_recv() {
                answer = Some(r);
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let ok = matches!(&answer, Some(Reply::Answered { text }) if text == "42");
        assert!(
            ok,
            "expected ANSWERED 42 from the live responder, got {answer:?}"
        );
        assert!(app.pending_asks.is_empty(), "resolved ask is cleared");
    }
}
