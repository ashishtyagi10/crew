//! Top-level render entry point. Early-returns for full-screen overlays,
//! computes the main layout, paints panels via `render_panel_leaves`, then
//! command-line/feedback/fn-bar, then `render_overlays`.

mod overlays;
mod panels;

use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use farx_core::PanelSide;

use crate::components::command_line;
use crate::components::diff_view::render_diff_view;
use crate::components::editor::render_editor;
use crate::components::feedback::render_feedback;
use crate::components::help::render_help;
use crate::components::slash_suggestions::render_slash_suggestions;
use crate::components::viewer::render_viewer;

/// Rect for the centered command box on the empty canvas: ~60% width
/// (clamped 40..100 cols), horizontally centered, around vertical middle.
fn centered_command_rect(area: Rect) -> Rect {
    let w = (area.width.saturating_mul(6) / 10)
        .clamp(40, 100)
        .min(area.width);
    let x = area.x + area.width.saturating_sub(w) / 2;
    let y = area.y + area.height / 2;
    Rect {
        x,
        y: y.saturating_sub(1),
        width: w,
        height: 3,
    }
}

/// Dim one-line hint shown under the empty-canvas command box.
fn render_launcher_hint(frame: &mut Frame, cmd_rect: Rect) {
    let area = Rect {
        x: cmd_rect.x,
        y: cmd_rect.y.saturating_add(cmd_rect.height),
        width: cmd_rect.width,
        height: 1,
    };
    let hint = Paragraph::new("/claude   /codex   /shell   ·   Alt+Enter to focus")
        .style(Style::default().fg(Color::Indexed(244)))
        .alignment(Alignment::Center);
    frame.render_widget(hint, area);
}

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
            ])
            .split(size);

        self.render_status_bar(frame, main_chunks[1]);

        self.render_agent_grid(frame, main_chunks[0]);
        self.cached_fn_bar_rect = None;

        let active_dir = self.active_tree_ref().root.clone();
        let empty_canvas = self.grid.is_empty();

        // The command input lives in a centered box on the empty canvas
        // (initial launcher), otherwise the full-width bottom row.
        let cmd_rect = if empty_canvas {
            centered_command_rect(main_chunks[0])
        } else {
            main_chunks[2]
        };

        if self.feedback.has_content() {
            render_feedback(frame, main_chunks[2], &self.feedback);
        }
        if empty_canvas || !self.feedback.has_content() {
            command_line::render_command_line(
                frame,
                cmd_rect,
                &self.command_line,
                &active_dir,
                &self.theme,
            );
            if empty_canvas && self.command_line.input.is_empty() {
                render_launcher_hint(frame, cmd_rect);
            }
        }

        if let Some(ref ss) = self.slash_suggestions {
            let ss_rect = if empty_canvas {
                Rect {
                    y: cmd_rect.y.saturating_add(cmd_rect.height),
                    ..cmd_rect
                }
            } else {
                main_chunks[2]
            };
            render_slash_suggestions(frame, ss, ss_rect);
        }

        self.render_overlays(frame, main_chunks[0]);
    }
}
