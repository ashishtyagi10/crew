use std::io::Write;

use crate::app::{CrewApp, FALLBACK_SIZE, GAP};
use crate::chat::ChatPane;
use crate::layout::Rect;
use crate::pane::{spawn_pane, Pane, PaneContent, TermPane};
use crew_plugin::{Plugin, PluginCommand};
use crew_term::{GridSize, PtyTerm};

impl CrewApp {
    /// Spawn a new terminal pane and focus it.
    pub fn spawn_new_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        match spawn_pane("bash", "sh", grid) {
            Ok(pane) => {
                self.panes.push(pane);
                self.focused = self.panes.len() - 1;
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
                let mut pane = Pane {
                    content: PaneContent::Terminal(Box::new(TermPane { pty, input })),
                    grid,
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: 0.0,
                        h: 0.0,
                    },
                    label: Some(label),
                };
                if let Some(renderer) = &self.renderer {
                    let (cell_w, cell_h) = renderer.cell_size();
                    let (sw, sh) = renderer.surface_size();
                    let n = self.panes.len() + 1;
                    let rects = crate::layout::pane_rects(n, sw as f32, sh as f32, GAP);
                    if let Some(r) = rects.last() {
                        pane.rect = *r;
                        let cols = ((r.w / cell_w).floor() as u16).max(1);
                        let rows = ((r.h / cell_h).floor() as u16).max(1);
                        pane.grid = GridSize { cols, rows };
                    }
                }
                self.panes.push(pane);
                self.focused = self.panes.len() - 1;
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
                });
                self.focused = self.panes.len() - 1;
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
