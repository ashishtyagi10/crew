//! A canvas's life: making one, closing one, and the two things every canvas must agree on.
//!
//! Split from [`super`] for size. What is here is what a SECOND window forced up out of
//! [`crate::app::CrewApp`] — the config is about the user rather than about a window, and a
//! session with two windows has to come back as two windows.
use winit::event_loop::ActiveEventLoop;

use super::Crew;
use crate::app::CrewApp;

impl Crew {
    /// A new canvas: the shared config and the active canvas's directory, and
    /// nothing else — no panes, its own focus, its own everything. It is not
    /// the first, so the launch notes and upgrade migrations are not repeated.
    pub(super) fn fresh(&self) -> CrewApp {
        let cwd = self
            .canvases
            .get(self.active)
            .map(|c| c.cwd.clone())
            .unwrap_or_default();
        CrewApp {
            config: self.config.clone(),
            cwd: cwd.clone(),
            // An empty canvas has nothing else to type into.
            input: crate::inputbar::InputBar {
                focused: true,
                history: crate::history::load(),
                cwd,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Close the canvases that asked to go. Closing the LAST one quits —
    /// there is no crew without a window — and closing any other does not.
    pub(super) fn close_asked(&mut self, event_loop: &ActiveEventLoop) {
        if !self.canvases.iter().any(|c| c.closing) {
            return;
        }
        if self.canvases.len() <= 1 {
            self.save_all();
            event_loop.exit();
            return;
        }
        self.canvases.retain(|c| !c.closing);
        self.active = self.active.min(self.canvases.len() - 1);
    }

    /// Publish a config the active canvas changed to every other canvas.
    ///
    /// The config is one thing about the *user*, not about a window: change
    /// the font in one and the other must not go on drawing at the old size
    /// and then save the old value over yours. Compared rather than
    /// subscribed to, because every path that writes it already exists and
    /// none of them knows there is more than one canvas.
    pub(super) fn share_config(&mut self) {
        let Some(active) = self.canvases.get(self.active) else {
            return;
        };
        if active.config == self.config {
            return;
        }
        self.config = active.config.clone();
        let shared = self.config.clone();
        for (i, c) in self.canvases.iter_mut().enumerate() {
            if i != self.active {
                c.apply_config(shared.clone());
            }
        }
    }

    /// Save every canvas's panes as one session, each stamped with the window
    /// it was in — so a session with two windows comes back as two windows.
    pub(super) fn save_all(&mut self) {
        let mut all = Vec::new();
        let mut restorable = false;
        for (i, c) in self.canvases.iter().enumerate() {
            restorable |= c.had_restorable;
            all.extend(c.session_panes().into_iter().map(|mut p| {
                p.window = i;
                p
            }));
        }
        if !all.is_empty() || restorable {
            crate::sessionsave::save_at(crate::sessionsave::path(), all);
        }
    }
}
