//! `/restart`, from the only place that can see every window.
//!
//! A canvas asks ([`CrewApp::want_restart`]); this answers. The order is the
//! whole design: the snapshot is written BEFORE the fresh process is spawned,
//! because that process reads it as it starts and would otherwise race the
//! one writing it — and this process only winds down once the spawn worked,
//! so a failed relaunch costs nothing but a status line.
use winit::event_loop::ActiveEventLoop;

use super::Crew;
use crate::app::CrewApp;
use crate::restartsnap;

impl Crew {
    /// Answer a canvas that asked to restart: snapshot every window, launch
    /// the fresh crew, and wind this one down.
    pub(super) fn restart_asked(&mut self, event_loop: &ActiveEventLoop) {
        let Some(i) = self.canvases.iter().position(|c| c.want_restart) else {
            return;
        };
        self.canvases[i].want_restart = false;
        let windows = self.canvases.iter().map(CrewApp::restart_window).collect();
        restartsnap::save_at(restartsnap::path(), windows);
        match crate::detach::spawn_detached_copy() {
            Ok(_) => {
                self.restarting = true;
                event_loop.exit();
            }
            Err(e) => {
                // Nothing will read it, and the next ordinary launch must not
                // mistake it for a restart of its own.
                restartsnap::discard_at(restartsnap::path());
                self.canvases[i].set_status(format!("restart failed: {e}"));
            }
        }
    }

    /// The other side: a launch that finds a restart snapshot puts every
    /// window back. The first window's panes go into the canvas the launch
    /// already opened; each further window gets a canvas of its own.
    pub(super) fn resume_restart(&mut self, event_loop: &ActiveEventLoop) {
        // No renderer means the GPU never came up and this launch is already
        // exiting — leave the snapshot for one that can use it.
        if self.canvases.first().is_none_or(|c| c.renderer.is_none()) {
            return;
        }
        let mut windows = restartsnap::take_at(restartsnap::path()).into_iter();
        let Some(first) = windows.next() else { return };
        let mut opened = self.canvases[0].resume_window(first);
        for win in windows {
            let mut next = self.fresh();
            next.open_window(event_loop);
            opened += next.resume_window(win);
            self.canvases.push(next);
        }
        let first = &mut self.canvases[0];
        // Held for the first frame, like every launch note — see `pending_note`.
        let note = resumed_note(opened);
        first.pending_note = Some(match first.pending_note.take() {
            Some(n) => format!("{n} \u{b7} {note}"),
            None => note,
        });
    }
}

/// What the first frame after a restart says.
pub(super) fn resumed_note(opened: usize) -> String {
    let v = env!("CARGO_PKG_VERSION");
    match opened {
        0 => format!("restarted \u{2014} crew v{v}"),
        1 => format!("restarted \u{2014} crew v{v}, 1 pane resumed"),
        n => format!("restarted \u{2014} crew v{v}, {n} panes resumed"),
    }
}
