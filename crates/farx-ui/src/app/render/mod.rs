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

/// FARX banner + hint, centered on the empty canvas (no agents running).
fn render_farx_logo(frame: &mut Frame, area: Rect) {
    const ART: [&str; 5] = [
        "███████  █████  ██████  ██   ██",
        "██      ██   ██ ██   ██  ██ ██ ",
        "█████   ███████ ██████    ███  ",
        "██      ██   ██ ██   ██  ██ ██ ",
        "██      ██   ██ ██   ██ ██   ██",
    ];
    let art_w = ART[0].chars().count() as u16;
    if area.width < art_w || area.height < ART.len() as u16 + 2 {
        return;
    }
    let x = area.x + (area.width - art_w) / 2;
    let y = area.y + area.height / 3;
    for (i, line) in ART.iter().enumerate() {
        let row = Rect {
            x,
            y: y + i as u16,
            width: art_w,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(*line).style(Style::default().fg(Color::Indexed(75))),
            row,
        );
    }
    let hint = Rect {
        x: area.x,
        y: y + ART.len() as u16 + 1,
        width: area.width,
        height: 1,
    };
    frame.render_widget(
        Paragraph::new("/claude   /codex   /shell   ·   F1 to focus input   ·   /exit to quit")
            .style(Style::default().fg(Color::Indexed(244)))
            .alignment(Alignment::Center),
        hint,
    );
}

/// Inset a rect horizontally (~8%, clamped 2..6 cols) so a box doesn't touch
/// the screen edges.
fn inset_x(area: Rect) -> Rect {
    let m = (area.width / 12).clamp(2, 6);
    Rect {
        x: area.x + m,
        y: area.y,
        width: area.width.saturating_sub(m * 2),
        height: area.height,
    }
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
            let status = self.agent_status_text();
            command_line::render_command_line(
                frame,
                size,
                &self.command_line,
                &active_dir,
                &status,
                &self.theme,
            );
            return;
        }

        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(size);

        self.render_agent_grid(frame, main_chunks[0]);

        // On the empty canvas, show the FARX banner above the input.
        if self.grid.is_empty() {
            render_farx_logo(frame, main_chunks[0]);
        }

        // The command input sits in the bottom row, inset from the screen
        // edges. Its footer carries the agent status ("no agents" / "N agents").
        let cmd_area = inset_x(main_chunks[1]);
        if self.feedback.has_content() {
            render_feedback(frame, cmd_area, &self.feedback);
        } else {
            let active_dir = self.active_tree_ref().root.clone();
            let status = self.agent_status_text();
            command_line::render_command_line(
                frame,
                cmd_area,
                &self.command_line,
                &active_dir,
                &status,
                &self.theme,
            );
        }

        if let Some(ref ss) = self.slash_suggestions {
            render_slash_suggestions(frame, ss, cmd_area);
        }

        self.render_overlays(frame, main_chunks[0]);
    }
}
