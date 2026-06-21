use std::sync::Arc;

use winit::event::Modifiers;
use winit::window::Window;

use crate::session::grid_for;
use crew_render::Renderer;
use crew_term::GridSize;

use crate::pane::Pane;

/// Fallback grid size when the GPU cell size is not yet known (zero).
pub(crate) const FALLBACK_SIZE: GridSize = GridSize { cols: 80, rows: 24 };
pub(crate) const POLL_MS: u64 = 16;
pub(crate) const GAP: f32 = 4.0;

#[derive(Default)]
pub struct CrewApp {
    pub(crate) window: Option<Arc<Window>>,
    pub(crate) renderer: Option<Renderer>,
    pub(crate) panes: Vec<Pane>,
    pub(crate) focused: usize,
    pub(crate) mods: Modifiers,
    pub(crate) cursor: (f32, f32),
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
            return true;
        }
        self.focused = self.focused.min(self.panes.len() - 1);
        false
    }

    /// Handle a Super-chord key.  Returns `true` if the app should exit.
    pub(crate) fn handle_super_chord(&mut self, s: &str) -> bool {
        let n = self.panes.len().max(1);
        match s {
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
            "[" => self.focused = (self.focused + n - 1) % n,
            "]" => self.focused = (self.focused + 1) % n,
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

    pub(crate) fn redraw(&self) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}
