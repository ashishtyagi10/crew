use std::io::Write;

use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::chat::ChatPane;
use crate::config::CrewConfig;
use crate::layout::Rect;
use crate::pane::{spawn_pane, Pane, PaneContent, TermPane};
use crate::settingspane::SettingsPane;
use crew_plugin::{Plugin, PluginCommand};
use crew_term::PtyTerm;

/// The user's preferred shell from `$SHELL`, falling back to `/bin/sh`.
pub(crate) fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string())
}

impl CrewApp {
    /// Spawn a new terminal pane and focus it.
    pub fn spawn_new_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let shell = default_shell();
        match spawn_pane(&shell, "/bin/sh", grid) {
            Ok(pane) => {
                self.panes.push(pane);
                self.focus_new_pane();
            }
            Err(e) => eprintln!("spawn_new_pane failed: {e:#}"),
        }
    }

    /// Spawn a labeled terminal pane running `command args` and focus it.
    pub fn spawn_labeled_terminal(&mut self, command: &str, args: &[String], label: String) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        match PtyTerm::spawn_args(grid, command, args) {
            Ok(pty) => {
                let input = pty.writer();
                // rect/grid are placeholders; build_frame's relayout sizes the pane
                // to the content area (right of the sidebar) on the next frame.
                let pane = Pane {
                    content: PaneContent::Terminal(Box::new(TermPane { pty, input })),
                    grid,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.0,
                        h: 0.0,
                    },
                    label: Some(label),
                    activity: false,
                };
                self.panes.push(pane);
                self.focus_new_pane();
                self.redraw();
            }
            Err(e) => eprintln!("spawn_labeled_terminal failed: {e:#}"),
        }
    }

    /// Send `text + newline` to the pane labeled `label` (if Terminal).
    pub fn send_to_label(&mut self, label: &str, text: &str) {
        for pane in &mut self.panes {
            if pane.label.as_deref() == Some(label) {
                if let PaneContent::Terminal(t) = &mut pane.content {
                    if let Err(e) = t
                        .input
                        .write_all(text.as_bytes())
                        .and_then(|_| t.input.write_all(b"\n"))
                        .and_then(|_| t.input.flush())
                    {
                        eprintln!("send_to_label write error: {e}");
                    }
                }
                return;
            }
        }
    }

    /// Spawn a settings pane showing the app config and focus it.
    pub(crate) fn spawn_settings_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let families = self
            .renderer
            .as_ref()
            .map(|r| r.monospace_families())
            .unwrap_or_default();
        self.panes.push(Pane {
            content: PaneContent::Settings(SettingsPane::new(self.config.clone(), families)),
            grid,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            activity: false,
        });
        self.focus_new_pane();
    }

    /// Apply updated config: set font family + size live, persist to disk, and redraw.
    pub(crate) fn apply_settings(&mut self, cfg: CrewConfig) {
        self.config = cfg;
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        if let Some(r) = &mut self.renderer {
            r.set_font_family(self.config.font_family.clone());
            r.set_font_size(self.config.font_size * scale);
        }
        self.config.save();
        self.redraw();
    }

    /// Set the font size (clamped to the config's valid range), applying it live
    /// and persisting — shared by the Cmd+= / Cmd+- / Cmd+0 zoom chords.
    pub(crate) fn set_font(&mut self, size: f32) {
        let mut cfg = self.config.clone();
        cfg.font_size = size;
        self.apply_settings(cfg.clamped());
    }

    /// Spawn a new chat pane backed by the plugin at `cmd`.
    pub fn spawn_chat_pane(&mut self, cmd: &str) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        match Plugin::spawn(cmd, &[]) {
            Ok(mut plugin) => {
                if let Err(e) = plugin.send(&PluginCommand::Hello { v: 1 }) {
                    eprintln!("spawn_chat_pane: plugin hello error: {e}");
                }
                let chat = ChatPane::new(plugin, String::new());
                self.panes.push(Pane {
                    content: PaneContent::Chat(chat),
                    grid,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.0,
                        h: 0.0,
                    },
                    label: None,
                    activity: false,
                });
                self.focus_new_pane();
            }
            Err(e) => eprintln!("spawn_chat_pane failed: {e:#}"),
        }
    }

    /// Resolve the echo plugin command path.
    pub(crate) fn echo_plugin_cmd() -> String {
        std::env::var("CREW_CHAT_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("crew-echo-plugin")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "crew-echo-plugin".to_string())
        })
    }

    /// Resolve the orchestrator plugin command path.
    pub(crate) fn orchestrator_plugin_cmd() -> String {
        std::env::var("CREW_ORCHESTRATOR_PLUGIN").unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("crew-orchestrator-plugin")))
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| "crew-orchestrator-plugin".to_string())
        })
    }
}
