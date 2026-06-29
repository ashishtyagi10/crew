//! The per-tick background work driven by winit's `about_to_wait`: drain every
//! pane's PTY/plugin output, refresh the sidebar, animate the welcome screen,
//! reap exited shells, run host actions, and honour OSC 52 clipboard requests.
use std::time::{Duration, Instant};

use winit::event_loop::{ActiveEventLoop, ControlFlow};

use crate::app::{CrewApp, POLL_MS};
use crate::pane::PaneContent;

/// Poll ticks per rendered frame of the busy progress sweep: the loop runs at
/// ~62 Hz, so redrawing every 4th tick animates the sweep at ~15 fps.
const BUSY_ANIM_DIV: u64 = 4;

impl CrewApp {
    /// One poll cycle. Schedules the next wake-up before returning.
    pub(crate) fn poll_panes(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        // Drain EVERY pane each tick. A `for` loop (not `any()`/`fold`) so all
        // panes are polled for their side effects — `any()` would short-circuit
        // and starve later panes when an earlier one has output.
        let mut any_changed = false;
        let mut collected_actions = Vec::new();
        let focused = self.focused;
        for (i, p) in self.panes.iter_mut().enumerate() {
            let mut rang = false;
            let changed = match &mut p.content {
                PaneContent::Terminal(t) => {
                    let n = t.pty.try_read() > 0;
                    rang = t.pty.take_bell();
                    n
                }
                PaneContent::Chat(c) => {
                    let result = c.poll();
                    collected_actions.extend(result.actions);
                    result.changed
                }
                PaneContent::Swarm(s) => s.poll(),
                PaneContent::Settings(_) | PaneContent::Far(_) => false,
            };
            // Output / bells in a pane you're not watching flag it.
            if i != focused {
                p.activity |= changed;
                p.bell |= rang;
            }
            any_changed |= changed || rang;
        }
        if self.sidebar.refresh(&self.cwd) {
            any_changed = true;
        }
        // Clear a status message once it has aged out, repainting the border.
        if self.expire_status() {
            any_changed = true;
        }
        // Animate the matrix-rain welcome screen while there are no panes — but
        // only redraw every Nth tick, so the idle screen runs at ~20 fps, not 60.
        if self.panes.is_empty() {
            self.tick = self.tick.wrapping_add(1);
            if crate::welcome::anim_should_redraw(self.tick) {
                any_changed = true;
            }
        } else if self.panes.iter().any(crate::paneview::pane_busy) {
            // Drive the indeterminate progress sweep while any pane is busy,
            // throttled to ~15 fps so a working pane stays lively without
            // spinning the CPU. Idle (no busy pane) → no extra redraws at all.
            self.tick = self.tick.wrapping_add(1);
            if self.tick.is_multiple_of(BUSY_ANIM_DIV) {
                any_changed = true;
            }
        }
        // Close terminal panes whose shell has exited (e.g. the user typed `exit`).
        let exited: Vec<usize> = self
            .panes
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(&p.content, PaneContent::Terminal(t) if t.pty.exited()))
            .map(|(i, _)| i)
            .collect();
        if !exited.is_empty() {
            for i in exited.into_iter().rev() {
                self.close_pane(i);
            }
            any_changed = true;
        }
        let actions_ran = !collected_actions.is_empty();
        for action in collected_actions {
            use crate::chat::HostAction;
            match action {
                HostAction::SpawnPane {
                    command,
                    args,
                    label,
                } => self.spawn_labeled_terminal(&command, &args, label),
                HostAction::SendPane { label, text } => self.send_to_label(&label, &text),
            }
        }
        if any_changed || actions_ran {
            self.redraw();
        }
        // Honour OSC 52 copy requests from terminal programs.
        if let Some(text) = self.take_pane_clipboard() {
            if let Ok(mut cb) = arboard::Clipboard::new() {
                let _ = cb.set_text(text);
            }
        }
        self.sync_window_title();
        self.save_window_size_if_settled();

        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(POLL_MS),
        ));
    }

    /// Persist the window size once resizing has settled (~400ms idle), so a
    /// drag produces a single write rather than one per frame.
    fn save_window_size_if_settled(&mut self) {
        if let Some(t) = self.resize_at {
            if t.elapsed() >= Duration::from_millis(400) {
                self.config.save();
                self.resize_at = None;
            }
        }
    }
}
