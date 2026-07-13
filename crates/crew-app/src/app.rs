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
    /// Last resolved (first_word, verdict) — see [`Self::check_command`].
    pub(crate) cmd_cache: Option<(String, crate::cmdcheck::Verdict)>,
    pub(crate) config: CrewConfig,
    pub(crate) sidebar: Box<StatsPane>,
    /// Resolves each terminal pane's foreground PID to a command name for its
    /// title (e.g. `claude`), refreshed ~1×/s.
    pub(crate) procnames: crate::procname::ProcNames,
    /// `/font random` rotation state (pool cache + 10-minute clock).
    pub(crate) font_rotate: crate::fontrotate::FontRotate,
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
    /// Whether a restorable pane (shell / Far / crew chat) ever existed this
    /// session — gates the quit snapshot so a pane-less run can't wipe a
    /// saved `/restore` session.
    pub(crate) had_restorable: bool,
    /// Saved-session shell count for the welcome screen's `/restore` hint
    /// (seeded at startup, cleared once `/restore` spends the snapshot).
    pub(crate) restore_hint: Option<usize>,
    /// In-progress background self-update (`/update`): drives the left-nav UPDATE
    /// card and the auto-restart. `None` when no update is running.
    pub(crate) update: Option<crate::update::UpdateState>,
    /// In-flight `?` ask (AI command suggestion) on a worker thread. `None`
    /// when idle. See [`crate::askbar`].
    pub(crate) ask: Option<crate::askbar::Ask>,
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
        // Never let the clamp land focus on a pane minimized into the nav —
        // reconcile_grid would silently restore it. Prefer a visible pane;
        // with none left, the input bar takes focus and the pane stays tucked.
        if self.panes[self.focused].hidden {
            match self.nearest_visible(self.focused) {
                Some(i) => self.focused = i,
                None => self.input.focused = true,
            }
        }
        false
    }

    /// The non-hidden pane index nearest to `idx`, if any pane is visible.
    pub(crate) fn nearest_visible(&self, idx: usize) -> Option<usize> {
        (0..self.panes.len())
            .filter(|&i| !self.panes[i].hidden)
            .min_by_key(|&i| i.abs_diff(idx))
    }

    /// Keep the grid LRU in step with `self.panes` and the current focus. Adds
    /// any visible pane index not yet tracked (newly spawned), drops hidden and
    /// stale indices, and marks the focused pane most-recently-active. Called
    /// once per frame from `build_frame`.
    pub(crate) fn reconcile_grid(&mut self) {
        let n = self.panes.len();
        // Keyboard-focusing a hidden pane restores it — the one rule that makes
        // every focus path (nav-row click, Cmd+N, spawn) a restore path. The
        // input bar holding focus means no pane is active, so nothing restores.
        if !self.input.focused {
            if let Some(p) = self.panes.get_mut(self.focused) {
                p.hidden = false;
            }
        }
        // Hidden panes leave the grid without reindexing — a hide keeps the
        // panes vec intact, unlike a close. Also drops any stale index past the
        // end (defensive; close_pane already fixes the common case via on_close).
        let panes = &self.panes;
        self.grid
            .retain(|i| panes.get(i).is_some_and(|p| !p.hidden));
        for idx in 0..n {
            if !self.panes[idx].hidden
                && !self.grid.full().contains(&idx)
                && !self.grid.minimized().contains(&idx)
            {
                self.grid.add(idx);
            }
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
        // `*text` broadcasts one line to every terminal pane, explicitly — the
        // bar's replacement for depending on Cmd+S broadcast mode.
        if let Some(cmd) = star_command(&line) {
            if cmd.is_empty() {
                self.set_status("usage: *<text> — sends to every terminal");
            } else if self.write_terminal_targets(&submit_bytes(cmd), true) == 0 {
                self.set_status("no terminals to broadcast to");
            }
            return false;
        }
        // `??question` asks the AI to explain the focused pane's output; the
        // answer opens in the zoomed markdown viewer. Checked before `?` —
        // qmark_command would read `??x` as an ask for "?x".
        if let Some(question) = crate::askbar::explain_command(&line) {
            self.start_explain(question);
            return false;
        }
        // `?query` asks the AI for a shell command (à la Warp AI); the reply
        // lands back in the input bar, ready to edit or Enter.
        if let Some(query) = crate::askbar::qmark_command(&line) {
            if query.is_empty() {
                self.set_status("usage: ?<what you want> — ask ai for a command");
            } else {
                self.start_ask(query);
            }
            return false;
        }
        // `cd` in the input bar moves Crew's working directory, not the terminal's.
        if self.try_change_dir(&line) {
            return false;
        }
        match crate::route::route_bare(self.focused_target(), &self.check_command(&line)) {
            crate::route::BareRoute::TypeInto(_) => {
                // The focused idle shell receives the line as keystrokes.
                if self.write_terminal_targets(&submit_bytes(&line), false) == 0 {
                    self.set_status("no shell here — press Cmd+T to open one");
                }
            }
            crate::route::BareRoute::Spawn => self.run_in_pane(&line),
            crate::route::BareRoute::BuiltinHint(b) => {
                self.set_status(format!(
                    "{b} is a shell builtin — run it inside a shell pane"
                ));
            }
            crate::route::BareRoute::UnknownHint => {
                self.set_status(format!("not a command — !{line} runs it in a pane anyway"));
            }
        }
        false
    }

    /// The focused pane as routing sees it: `IdleShell` only for a visible
    /// terminal whose shell owns the prompt (`foreground_pid()` is `None`).
    /// Hidden panes are not "in the main area", so they never receive text.
    pub(crate) fn focused_target(&self) -> crate::route::Target {
        if let Some(p) = self.panes.get(self.focused) {
            if !p.hidden {
                if let crate::pane::PaneContent::Terminal(t) = &p.content {
                    if t.pty.foreground_pid().is_none() {
                        return crate::route::Target::IdleShell(self.focused);
                    }
                }
            }
        }
        crate::route::Target::Other
    }

    /// Resolve `line`'s first word, memoized — the palette preview re-checks on
    /// every keystroke and only the first word matters, so argument typing must
    /// not re-stat the PATH.
    pub(crate) fn check_command(&mut self, line: &str) -> crate::cmdcheck::Verdict {
        let word = crate::cmdcheck::first_word(line);
        if let (Some(w), Some((cached_w, v))) = (&word, &self.cmd_cache) {
            if w == cached_w {
                return v.clone();
            }
        }
        let v = crate::cmdcheck::resolve(line, &crate::cmdcheck::effective_path());
        if let Some(w) = word {
            self.cmd_cache = Some((w, v.clone()));
        }
        v
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

/// If `line` is a `*text` broadcast, return the trimmed payload (empty when
/// just `*`); else `None`. The payload is sent to EVERY terminal pane —
/// broadcast is an explicit prefix, not a mode, so nothing else the bar does
/// depends on Cmd+S state.
pub(crate) fn star_command(line: &str) -> Option<&str> {
    line.strip_prefix('*').map(str::trim)
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
mod unit_tests {
    use super::star_command;

    #[test]
    fn star_command_strips_the_prefix() {
        assert_eq!(star_command("* ls -la"), Some("ls -la"));
        assert_eq!(star_command("*ls"), Some("ls"));
        assert_eq!(star_command("*"), Some(""));
        assert_eq!(star_command("ls *"), None);
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
