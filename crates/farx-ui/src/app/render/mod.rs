//! Top-level render entry point. Early-returns for full-screen overlays,
//! computes the main layout, paints panels via `render_panel_leaves`, then
//! command-line/feedback/fn-bar, then `render_overlays`.

mod overlays;
mod panels;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use farx_core::PanelSide;

use crate::components::command_line;
use crate::components::diff_view::render_diff_view;
use crate::components::editor::render_editor;
use crate::components::feedback::render_feedback;
use crate::components::fn_bar;
use crate::components::help::render_help;
use crate::components::slash_suggestions::render_slash_suggestions;
use crate::components::viewer::render_viewer;

use super::App;

impl App {
    /// Paint one frame.
    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.area();

        if let Some(ref mut editor) = self.editor {
            render_editor(frame, editor, &self.theme);
            return;
        }
        if let Some(ref mut viewer) = self.viewer {
            render_viewer(frame, viewer, &self.theme);
            return;
        }
        if let Some(ref help) = self.help {
            render_help(frame, help, &self.theme);
            return;
        }
        if let Some(ref diff) = self.diff_view {
            render_diff_view(frame, diff, &self.theme);
            return;
        }

        if !self.panels_visible {
            let active_dir = match self.active_panel {
                PanelSide::Left => self.left_panel.current_dir.clone(),
                PanelSide::Right => self.right_panel.current_dir.clone(),
            };
            command_line::render_command_line(
                frame,
                size,
                &self.command_line,
                &active_dir,
                &self.theme,
            );
            return;
        }

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ])
            .split(size);

        self.render_status_bar(frame, main_chunks[1]);

        self.render_agent_grid(frame, main_chunks[0]);

        if self.config.ui.show_fn_bar {
            self.cached_fn_bar_rect = Some(main_chunks[3]);
        } else {
            self.cached_fn_bar_rect = None;
        }

        if self.feedback.has_content() {
            render_feedback(frame, main_chunks[2], &self.feedback);
        } else {
            let active_dir = self.active_tree_ref().root.clone();
            command_line::render_command_line(
                frame,
                main_chunks[2],
                &self.command_line,
                &active_dir,
                &self.theme,
            );
        }

        if let Some(ref ss) = self.slash_suggestions {
            render_slash_suggestions(frame, ss, main_chunks[2]);
        }

        if self.config.ui.show_fn_bar {
            fn_bar::render_fn_bar(frame, main_chunks[3], &self.theme);
        }

        self.render_overlays(frame, main_chunks[0]);
    }
}
