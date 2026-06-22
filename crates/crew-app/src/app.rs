use std::io::Write;
use std::sync::Arc;

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

    /// Handle a Super-chord key.  Returns `true` if the app should exit.
    pub(crate) fn handle_super_chord(&mut self, s: &str) -> bool {
        let n = self.panes.len().max(1);
        match s {
            "i" => self.input.focused = !self.input.focused,
            "," => self.spawn_settings_pane(),
            "g" => self.toggle_sidebar(),
            "t" => self.spawn_new_pane(),
            "j" => {
                let cmd = Self::echo_plugin_cmd();
                self.spawn_chat_pane(&cmd);
            }
            "o" => {
                let cmd = Self::orchestrator_plugin_cmd();
                self.spawn_chat_pane(&cmd);
            }
            "w" => return self.close_pane(self.focused),
            "m" => {
                if let Some(w) = &self.window {
                    w.set_maximized(!w.is_maximized());
                }
            }
            "[" => self.focused = (self.focused + n - 1) % n,
            "]" => self.focused = (self.focused + 1) % n,
            "z" => self.zoomed = !self.zoomed,
            // Font zoom: Cmd+= / Cmd+- grow/shrink, Cmd+0 resets to default.
            "=" | "+" => self.set_font(self.config.font_size + 1.0),
            "-" | "_" => self.set_font(self.config.font_size - 1.0),
            "0" => self.set_font(14.0),
            s if s.len() == 1 => {
                if let Some(d) = s.chars().next().and_then(|c| c.to_digit(10)) {
                    if d >= 1 {
                        let i = (d - 1) as usize;
                        if i < self.panes.len() {
                            self.focused = i;
                        }
                    }
                }
            }
            _ => {}
        }
        false
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
        let focused = self.focused;
        if let Some(pane) = self.panes.get_mut(focused) {
            if let PaneContent::Terminal(t) = &mut pane.content {
                if let Err(e) = t
                    .input
                    .write_all(line.as_bytes())
                    .and_then(|_| t.input.write_all(b"\n"))
                    .and_then(|_| t.input.flush())
                {
                    eprintln!("submit_input write error: {e}");
                }
            }
        }
        false
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

    /// Set the font size (clamped to the config's valid range), applying it live
    /// and persisting — shared by the Cmd+= / Cmd+- / Cmd+0 zoom chords.
    pub(crate) fn set_font(&mut self, size: f32) {
        let mut cfg = self.config.clone();
        cfg.font_size = size;
        self.apply_settings(cfg.clamped());
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
