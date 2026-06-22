use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use winit::event::Modifiers;
use winit::window::Window;

use crate::config::CrewConfig;
use crate::inputbar::InputBar;
use crate::pane::{Pane, PaneContent};
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
    pub(crate) mods: Modifiers,
    pub(crate) cursor: (f32, f32),
    pub(crate) config: CrewConfig,
    pub(crate) sidebar: Box<StatsPane>,
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
        }
        if self.panes.is_empty() {
            // No panel selected → focus returns to the input bar.
            self.focused = 0;
            self.input.focused = true;
            return false;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        false
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
        let mut bytes = line.into_bytes();
        bytes.push(b'\n');
        self.write_to_terminals(&bytes);
        false
    }

    /// Write `bytes` to the focused terminal — or, when broadcast is on, to every
    /// terminal pane (tmux-style synchronized input). Each write snaps to bottom.
    pub(crate) fn write_to_terminals(&mut self, bytes: &[u8]) {
        let all = self.broadcast;
        let focused = self.focused;
        for (i, pane) in self.panes.iter_mut().enumerate() {
            if !all && i != focused {
                continue;
            }
            if let PaneContent::Terminal(t) = &mut pane.content {
                t.pty.scroll_to_bottom();
                if let Err(e) = t.input.write_all(bytes).and_then(|_| t.input.flush()) {
                    eprintln!("terminal write error: {e}");
                }
            }
        }
    }

    /// Run a `/command` typed in the input bar. Returns `true` if the app should exit.
    fn run_slash_command(&mut self, cmd: &str) -> bool {
        match cmd {
            "exit" => return true,
            "keys" => self.help_open = true,
            "settings" => self.spawn_settings_pane(),
            "shell" => self.spawn_new_pane(),
            "update" => self.spawn_labeled_terminal(
                "sh",
                &["-c".to_string(), "git pull; exec sh".to_string()],
                "update".to_string(),
            ),
            _ => {}
        }
        false
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

#[cfg(test)]
mod tests {
    use super::{slash_command, CrewApp};

    #[test]
    fn slash_command_parses() {
        assert_eq!(slash_command("/settings"), Some("settings"));
        assert_eq!(slash_command("/ settings "), Some("settings"));
        assert_eq!(slash_command("ls -la"), None);
        assert_eq!(slash_command("/"), Some(""));
    }

    #[test]
    fn zoom_chord_toggles() {
        let mut app = CrewApp::default();
        assert!(!app.zoomed);
        app.handle_super_chord("z");
        assert!(app.zoomed);
        app.handle_super_chord("z");
        assert!(!app.zoomed);
    }
}
