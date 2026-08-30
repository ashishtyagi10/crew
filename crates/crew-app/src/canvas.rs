//! Crew's windows, and which one an event belongs to.
//!
//! A crew window is a whole **canvas**: a grid of panes, a left nav, an input
//! bar, its own focus and its own zoom. This owns however many of those exist
//! and routes each event to the one whose window it came from.
//!
//! The canvas type is [`CrewApp`] itself — unchanged. That is the point of
//! the arrangement: everything per-window was already a field on it and every
//! method that reads `self.panes` is already a method *of one canvas*, so a
//! second window is a second `CrewApp` rather than two hundred call sites
//! learning which window they meant. What had to move up here is only what a
//! second one must NOT duplicate.
//!
//! **What is per-canvas:** panes, the grid, focus, zoom, the input bar, the
//! sidebar, toasts, hint mode, the glide clock, document windows.
//! **What is per-process:** the theme, the palette, motion, the usage ledger,
//! the todo store, the `crew ask` socket and the federation relay — every one
//! of which was already a global or is built once in `handler::run` and
//! handed to the first canvas. **What is per-process but lives in a field:**
//! the config, which is why [`Crew::share_config`] exists.
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::app::CrewApp;
use crate::config::CrewConfig;

/// Every open canvas.
pub struct Crew {
    canvases: Vec<CrewApp>,
    /// The canvas whose window last had an event — where a canvas-less action
    /// (a restart, a quit) is answered from.
    active: usize,
    /// The config as it was after the last event, to notice a canvas changing
    /// it (see [`Crew::share_config`]).
    config: CrewConfig,
}

impl Crew {
    pub fn new(first: CrewApp) -> Self {
        Self {
            config: first.config.clone(),
            canvases: vec![first],
            active: 0,
        }
    }

    /// The canvas an event was for, or the active one.
    fn at(&mut self, id: WindowId) -> Option<usize> {
        let found = self
            .canvases
            .iter()
            .position(|c| c.owns(id) || c.is_doc_window(id));
        if let Some(i) = found {
            self.active = i;
            // What the quit guard speaks for: Cmd+Q takes the app, so the
            // count that matters is every pane in every window.
            let others: usize = self
                .canvases
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, c)| c.panes.len())
                .sum();
            self.canvases[i].other_panes = others;
        }
        found
    }

    /// A canvas asked for another window (`Cmd+N`). Windows can only be made
    /// from a callback holding the active event loop, so the ask is a flag
    /// and this is where it is answered.
    fn open_asked(&mut self, event_loop: &ActiveEventLoop) {
        // A restored session can carry more than one window's panes; each
        // group opens a canvas of its own and is replayed into it.
        let groups: Vec<Vec<crate::sessionsave::SavedPane>> = self
            .canvases
            .iter_mut()
            .flat_map(|c| std::mem::take(&mut c.pending_windows))
            .collect();
        for group in groups {
            let mut next = self.fresh();
            next.open_window(event_loop);
            next.restore_from(group);
            self.canvases.push(next);
        }
        let asked = self
            .canvases
            .iter_mut()
            .any(|c| std::mem::take(&mut c.want_window));
        if asked {
            // The new canvas inherits the config and nothing else: no panes,
            // its own focus, its own everything. It is not the first, so the
            // launch-time notes and upgrade migrations are not repeated.
            let mut next = self.fresh();
            next.open_window(event_loop);
            self.canvases.push(next);
            self.active = self.canvases.len() - 1;
        }
        for c in self.canvases.iter_mut().filter(|c| c.window.is_none()) {
            c.open_window(event_loop);
        }
    }

    /// A new canvas: the shared config and the active canvas's directory, and
    /// nothing else — no panes, its own focus, its own everything. It is not
    /// the first, so the launch notes and upgrade migrations are not repeated.
    fn fresh(&self) -> CrewApp {
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
    fn close_asked(&mut self, event_loop: &ActiveEventLoop) {
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
    fn share_config(&mut self) {
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
    fn save_all(&mut self) {
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

impl ApplicationHandler for Crew {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.open_asked(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        for c in self.canvases.iter_mut() {
            c.tick(event_loop);
        }
        self.open_asked(event_loop);
        self.close_asked(event_loop);
        self.share_config();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(i) = self.at(id) else { return };
        self.canvases[i].route(event_loop, id, event);
        self.close_asked(event_loop);
        self.share_config();
    }

    /// Fires once when the event loop winds down (any quit path — Cmd+Q, the
    /// last window closing, `/exit`): snapshot the open shells' directories so
    /// `/restore` can reopen them next launch.
    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.save_all();
    }
}

#[cfg(test)]
#[path = "canvas_tests.rs"]
mod canvas_tests;
