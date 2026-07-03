use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use winit::event::Modifiers;
use winit::window::Window;

use crate::config::CrewConfig;
use crate::grid::GridLayout;
use crate::inputbar::InputBar;
use crate::pane::Pane;
use crate::session::grid_for;
use crate::statspane::StatsPane;
use crew_render::Renderer;
use crew_term::GridSize;

/// Fallback grid size when the GPU cell size is not yet known (zero).
pub(crate) const FALLBACK_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
pub(crate) const POLL_MS: u64 = 16;
pub(crate) const GAP: f32 = 8.0;

#[derive(Default)]
pub struct CrewApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) renderer: Option<Renderer>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) focused: usize,
    /// LRU of pane indices: which panes are full tiles vs. minimized.
    pub(crate) grid: GridLayout,
    pub(crate) mods: Modifiers,
    pub(crate) cursor: (f32, f32),
    /// Sub-line scroll remainder, in lines. Trackpads emit many small pixel
    /// deltas; we accumulate the fractional part here so slow scrolling adds up
    /// instead of each tick rounding to zero and being lost.
    pub(crate) scroll_accum: f32,
    pub(crate) config: CrewConfig,
    pub(crate) sidebar: Box<StatsPane>,
    /// Resolves each terminal pane's foreground PID to a command name for its
    /// title (e.g. `claude`), refreshed ~1×/s.
    pub(crate) procnames: crate::procname::ProcNames,
    pub(crate) input: InputBar,
    /// Animation frame counter, advanced while the welcome screen is showing.
    pub(crate) tick: u64,
    /// Whether the keybindings help overlay is showing.
    pub(crate) help_open: bool,
    /// Whether the focused pane is zoomed to fill the content area.
    pub(crate) zoomed: bool,
    /// Last OS window title set, to avoid redundant `set_title` calls.
    pub(crate) win_title: String,
    /// Mirror input to every terminal pane (tmux-style synchronized input).
    pub(crate) broadcast: bool,
    /// Time + pane index of the last left click, for double-click detection.
    pub(crate) last_click: Option<(Instant, usize)>,
    /// In-progress mouse drag selection over any pane, if any.
    pub(crate) drag: Option<crate::select::Drag>,
    /// Active text selection over a non-terminal pane (chat/settings/etc.),
    /// which lack alacritty's grid model. Persists after the drag so `Cmd+C`
    /// can copy it; cleared by the next press or a scroll. See [`crate::gridsel`].
    pub(crate) cell_sel: Option<crate::gridsel::CellSel>,
    /// Last `/find` term, so repeating it walks to the next older match.
    pub(crate) last_find: Option<String>,
    /// Crew's working directory: shown in the input-bar legend and used as the
    /// start directory for new shells. Moved by typing `cd` in the input bar.
    pub(crate) cwd: PathBuf,
    /// The directory before the last change, so `cd -` can toggle back.
    pub(crate) prev_cwd: PathBuf,
    /// When the window was last resized; drives a debounced save of its size.
    pub(crate) resize_at: Option<Instant>,
    /// Transient status message + when it was set, shown on the input bar.
    pub(crate) status: Option<(String, Instant)>,
    /// Ring buffer of recent status messages, shown as the live LOG section in
    /// the left nav (newest last). Capped at [`crate::status::LOG_CAP`].
    pub(crate) log: Vec<String>,
    /// Notification system: throttles + records pane events (command finished,
    /// bell, output pattern match, pane exit) surfaced via the LOG + input bar.
    pub(crate) notifier: crate::notify::Notifier,
    /// When quit was last pressed with panes open, for the confirm-to-quit window.
    pub(crate) quit_armed: Option<Instant>,
    /// In-progress background self-update (`/update`): drives the left-nav UPDATE
    /// card and the auto-restart. `None` when no update is running.
    pub(crate) update: Option<crate::update::UpdateState>,
}

impl CrewApp {
    pub(crate) fn current_grid(renderer: &Renderer) -> GridSize {
        let (cell_w, cell_h) = renderer.cell_size();
        if cell_w > 0.0 && cell_h > 0.0 {
            let (sw, sh) = renderer.surface_size();
            grid_for(sw, sh, cell_w, cell_h)
        } else {
            FALLBACK_SIZE
        }
    }

    /// Close pane at `idx`.  Returns `true` if the app should exit.
    pub fn close_pane(&mut self, idx: usize) -> bool {
        if idx < self.panes.len() {
            self.panes.remove(idx);
            self.grid.on_close(idx);
        }
        // Closing a pane returns to the grid; never linger zoomed on it.
        self.zoomed = false;
        if self.panes.is_empty() {
            // No panel selected → focus returns to the input bar; reset modes.
            self.focused = 0;
            self.input.focused = true;
            self.broadcast = false;
            self.input.broadcast = false;
            return false;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        false
    }

    /// Keep the grid LRU in step with `self.panes` and the current focus. Adds
    /// any pane index not yet tracked (newly spawned), drops any index past the
    /// end, and marks the focused pane most-recently-active. Called once per
    /// frame from `build_frame`.
    pub(crate) fn reconcile_grid(&mut self) {
        let n = self.panes.len();
        for idx in 0..n {
            if !self.grid.full().contains(&idx) && !self.grid.minimized().contains(&idx) {
                self.grid.add(idx);
            }
        }
        // Drop any stale indices at/after the end (defensive; close_pane already
        // fixes the common case via on_close). Terminates because each
        // `on_close(n)` removes/shifts the max stale index down toward `n`.
        while self.grid.len() > n {
            self.grid.on_close(n);
        }
        if n > 0 {
            self.grid.touch(self.focused.min(n - 1));
        }
    }

    /// Focus the most-recently-pushed pane and move keyboard focus off the input bar.
    pub(crate) fn focus_new_pane(&mut self) {
        self.focused = self.panes.len().saturating_sub(1);
        self.input.focused = false;
    }

    /// Handle a submitted input line: `/command`s are run; everything else is
    /// written (with a newline) to the focused Terminal pane. Returns `true` if the
    /// app should exit (e.g. `/exit`).
    pub(crate) fn submit_input(&mut self, line: String) -> bool {
        if line.is_empty() {
            return false;
        }
        if let Some(cmd) = slash_command(&line) {
            return self.run_slash_command(cmd);
        }
        // `!cmd` runs a shell command in its own pane (like `/run`), regardless of
        // which pane is focused — a quick `ls`/`git status` without leaving the
        // agent pane you're driving.
        if let Some(cmd) = bang_command(&line) {
            if cmd.is_empty() {
                self.set_status("usage: !<command>");
            } else {
                self.run_in_pane(cmd);
            }
            return false;
        }
        // `cd` in the input bar moves Crew's working directory, not the terminal's.
        if self.try_change_dir(&line) {
            return false;
        }
        let bytes = submit_bytes(&line);
        // Nothing received it (no terminal focused / open) — hint instead of a
        // silent no-op.
        if self.write_to_terminals(&bytes) == 0 {
            self.set_status("no shell here — press Cmd+T to open one");
        }
        false
    }

    /// Set (or, when `name` is empty, clear) the focused pane's title override.
    pub(crate) fn name_focused_pane(&mut self, name: &str) {
        if let Some(p) = self.panes.get_mut(self.focused) {
            p.name = (!name.is_empty()).then(|| name.to_string());
            self.redraw();
        } else {
            self.set_status("no pane to name");
        }
    }

    /// Toggle the window's maximized state and persist it.
    pub(crate) fn toggle_maximize(&mut self) {
        if let Some(w) = &self.window {
            let m = !w.is_maximized();
            w.set_maximized(m);
            self.config.maximized = m;
        }
        self.config.save();
    }

    pub(crate) fn toggle_sidebar(&mut self) {
        self.config.show_nav = !self.config.show_nav;
        self.config.save();
        self.redraw();
    }

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

/// If `line` is a `/command`, return the trimmed command name; else `None`.
pub(crate) fn slash_command(line: &str) -> Option<&str> {
    line.strip_prefix('/').map(str::trim)
}

/// If `line` is a `!command`, return the trimmed command (empty when just `!`);
/// else `None`. The command runs in its own pane via [`CrewApp::run_in_pane`].
pub(crate) fn bang_command(line: &str) -> Option<&str> {
    line.strip_prefix('!').map(str::trim)
}

/// Bytes to write when submitting an input-bar line to a terminal: the line
/// followed by a carriage return (0x0d) — the same byte a real Enter sends. A
/// trailing line feed (0x0a) is the Shift+Enter "soft return", which agent CLIs
/// (Claude/codex) treat as "insert a newline, keep editing", leaving the text
/// sitting highlighted in their input box instead of being submitted.
pub(crate) fn submit_bytes(line: &str) -> Vec<u8> {
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\r');
    bytes
}

/// Serialises tests that mutate crew-theme's process-global state (`CURRENT`,
/// the random-rotation atomics): several files across this crate exercise
/// `/theme` behaviour (chattheme.rs, toggles.rs, spawn.rs, config.rs) and
/// would otherwise race under the default parallel test runner. Mirrors the
/// `guard()` used by crew-theme's own tests.
#[cfg(test)]
pub(crate) fn theme_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
