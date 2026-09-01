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
mod ask;
mod life;

pub struct Crew {
    canvases: Vec<CrewApp>,
    /// The `crew ask` endpoint, held HERE rather than by the first canvas.
    ///
    /// It was the launch canvas's, which made every pane in every other window unaddressable:
    /// `crew panes` listed one window's panes and `crew ask` could only reach them. The socket
    /// is one thing about the PROCESS, so the process's owner holds it and routes each request
    /// to the canvas that can answer it.
    ipc: Option<crate::ipc::IpcHandle>,
    /// Broadcasts in flight, each fanned across every canvas and merged back into one reply.
    casts: Vec<ask::Merge>,
    /// The canvas whose window last had an event — where a canvas-less action
    /// (a restart, a quit) is answered from.
    active: usize,
    /// The config as it was after the last event, to notice a canvas changing
    /// it (see [`Crew::share_config`]).
    config: CrewConfig,
}

impl Crew {
    pub fn new(mut first: CrewApp) -> Self {
        Self {
            config: first.config.clone(),
            ipc: first.ipc.take(),
            canvases: vec![first],
            active: 0,
            casts: Vec::new(),
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
}

impl ApplicationHandler for Crew {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.open_asked(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.pump_ipc(crate::chattime::unix_now_ms());
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
