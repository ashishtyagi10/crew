//! Embedded-terminal lifecycle: spawn into a split, close + collapse, and
//! Tab/F4 cycle through file panels and terminals.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::components::embedded_terminal::{OutputWaker, TerminalSession};

use super::App;

impl App {
    /// Look up a terminal by stable id (linear scan).
    pub(crate) fn terminal_by_id(&self, id: usize) -> Option<&TerminalSession> {
        self.terminals.iter().find(|t| t.id == id)
    }

    /// Look up a terminal mutably by stable id (linear scan).
    pub(crate) fn terminal_by_id_mut(&mut self, id: usize) -> Option<&mut TerminalSession> {
        self.terminals.iter_mut().find(|t| t.id == id)
    }

    /// Register the event sender that PTY reader threads use to wake the
    /// render loop when an embedded terminal produces output.
    pub fn set_event_sender(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::event::Event>,
    ) {
        self.terminal_event_tx = Some(tx);
    }

    /// Build a waker that coalesces output notifications into a single queued
    /// [`crate::event::Event::TerminalOutput`] until the loop drains output.
    fn output_waker(&self) -> Option<OutputWaker> {
        let tx = self.terminal_event_tx.clone()?;
        let pending = self.output_pending.clone();
        Some(Arc::new(move || {
            if !pending.swap(true, Ordering::SeqCst) {
                let _ = tx.send(crate::event::Event::TerminalOutput);
            }
        }))
    }

    /// Spawn an embedded terminal in a new split panel.
    pub(super) fn spawn_embedded_terminal(&mut self, cmd: &str, args: &[&str]) {
        let dir = self.active_tree_ref().root.clone();
        let rows = 24;
        let cols = 80;
        let waker = self.output_waker();

        let terminal_id = self.next_terminal_id;
        self.next_terminal_id += 1;

        match TerminalSession::spawn(terminal_id, cmd, args, &dir, rows, cols, waker) {
            Ok(session) => {
                let title = session.title.clone();
                self.terminals.push(session);

                let leaves = self.layout.leaves();
                let focus_idx = if let Some(tid) = self.focused_terminal {
                    leaves
                        .iter()
                        .position(|l| *l == farx_core::PanelLeaf::Terminal(tid))
                        .unwrap_or(0)
                } else {
                    leaves
                        .iter()
                        .position(|l| *l == farx_core::PanelLeaf::FilePanel(self.active_panel))
                        .unwrap_or(0)
                };

                self.layout.split_leaf(focus_idx, terminal_id);

                let new_leaves = self.layout.leaves();
                if let Some(idx) = new_leaves
                    .iter()
                    .position(|l| *l == farx_core::PanelLeaf::Terminal(terminal_id))
                {
                    self.focused_terminal = Some(terminal_id);
                    let _ = idx;
                }

                self.grid.add(terminal_id);
                self.feedback
                    .info(format!("{} opened in split panel", title));
            }
            Err(e) => {
                self.feedback
                    .error(format!("Failed to spawn terminal: {}", e));
            }
        }
    }

    /// Close a terminal session and collapse its split.
    pub(super) fn close_terminal(&mut self, terminal_id: usize) {
        if self.terminal_by_id(terminal_id).is_none() {
            return;
        }

        self.layout.remove_terminal(terminal_id);
        self.terminals.retain(|t| t.id != terminal_id);
        self.grid.remove(terminal_id);

        if self.focused_terminal == Some(terminal_id) {
            self.focused_terminal = None;
        }
    }

    /// Cycle focus to the next agent tile — Tab / F4 key.
    /// Walks `self.grid.full()` then `self.grid.minimized()` (wrapping).
    /// Focusing a minimized id calls `self.grid.touch(id)` so it promotes
    /// into the full set on the next rendered frame.
    pub(super) fn cycle_focus(&mut self) {
        let order: Vec<usize> = self
            .grid
            .full()
            .iter()
            .chain(self.grid.minimized().iter())
            .copied()
            .collect();
        if order.is_empty() {
            self.focused_terminal = None;
            return;
        }
        let next = match self.focused_terminal {
            Some(cur) => {
                let i = order.iter().position(|x| *x == cur).unwrap_or(0);
                order[(i + 1) % order.len()]
            }
            None => order[0],
        };
        self.focused_terminal = Some(next);
        self.grid.touch(next);
        if let Some(t) = self.terminal_by_id_mut(next) {
            t.has_attention = false;
        }
    }

    /// Poll all terminal sessions for new output. Called on each tick and
    /// whenever a [`crate::event::Event::TerminalOutput`] wake-up arrives.
    pub fn poll_terminals(&mut self) {
        // Clear the coalescing flag first so output arriving during this drain
        // queues a fresh wake-up rather than being missed.
        self.output_pending.store(false, Ordering::SeqCst);
        let focused_tid = self.focused_terminal;
        for term in self.terminals.iter_mut() {
            let got_output = term.poll_output();
            if got_output && Some(term.id) != focused_tid {
                term.has_attention = true;
            }
        }
    }
}
