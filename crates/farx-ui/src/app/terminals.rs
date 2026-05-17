//! Embedded-terminal lifecycle: spawn into a split, close + collapse, and
//! Tab/F4 cycle through file panels and terminals.

use crate::components::embedded_terminal::TerminalSession;

use super::App;

impl App {
    /// Spawn an embedded terminal in a new split panel.
    pub(super) fn spawn_embedded_terminal(&mut self, cmd: &str, args: &[&str]) {
        let dir = self.active_tree_ref().root.clone();
        let rows = 24;
        let cols = 80;

        match TerminalSession::spawn(cmd, args, &dir, rows, cols) {
            Ok(session) => {
                let terminal_id = self.terminals.len();
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
        if terminal_id >= self.terminals.len() {
            return;
        }

        self.layout.remove_terminal(terminal_id);
        self.terminals.remove(terminal_id);
        self.layout.adjust_terminal_ids(terminal_id);

        match self.focused_terminal {
            Some(id) if id == terminal_id => {
                self.focused_terminal = None;
            }
            Some(id) if id > terminal_id => {
                self.focused_terminal = Some(id - 1);
            }
            _ => {}
        }
    }

    /// Cycle focus to the next panel (file or terminal) — Tab key.
    pub(super) fn cycle_focus(&mut self) {
        let leaves = self.layout.leaves();
        if leaves.is_empty() {
            return;
        }

        let current_idx = if let Some(tid) = self.focused_terminal {
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

        let next_idx = (current_idx + 1) % leaves.len();
        match leaves[next_idx] {
            farx_core::PanelLeaf::FilePanel(side) => {
                self.focused_terminal = None;
                self.active_panel = side;
            }
            farx_core::PanelLeaf::Terminal(tid) => {
                self.focused_terminal = Some(tid);
                if let Some(t) = self.terminals.get_mut(tid) {
                    t.has_attention = false;
                }
            }
        }
    }

    /// Poll all terminal sessions for new output. Called on each tick.
    pub fn poll_terminals(&mut self) {
        let focused_tid = self.focused_terminal;
        for (i, term) in self.terminals.iter_mut().enumerate() {
            let got_output = term.poll_output();
            if got_output && Some(i) != focused_tid {
                term.has_attention = true;
            }
        }
    }
}
