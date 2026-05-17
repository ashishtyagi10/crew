use ratatui::layout::Rect;
use ratatui::widgets::Clear;

use crate::theme::Theme;

use super::state::{DialogKind, DialogState};
use super::variants::{render_confirm, render_input, render_message};

pub fn render_dialog(frame: &mut ratatui::Frame, state: &DialogState, _theme: &Theme) {
    let area = frame.area();

    // Dialog size: centered, dynamically sized
    let dialog_width = match &state.kind {
        DialogKind::Message { message, .. } | DialogKind::Error { message, .. } => {
            // Size to content, min 60, max screen-4
            let max_line = message.lines().map(|l| l.len()).max().unwrap_or(20);
            (max_line as u16 + 4).clamp(60, area.width.saturating_sub(4))
        }
        _ => 60u16.min(area.width.saturating_sub(4)),
    };
    let dialog_height = match &state.kind {
        DialogKind::Input { .. } => 7,
        DialogKind::Confirm { details, .. } => {
            (7 + details.len() as u16).min(area.height.saturating_sub(4))
        }
        DialogKind::Message { message, .. } | DialogKind::Error { message, .. } => {
            // Count lines in message, +4 for borders + hint + padding
            let line_count = message.lines().count().max(1) as u16;
            (line_count + 4).clamp(7, area.height.saturating_sub(4))
        }
    };

    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    // Clear the area behind the dialog
    frame.render_widget(Clear, dialog_area);

    match &state.kind {
        DialogKind::Input {
            title,
            prompt,
            input,
            cursor_pos,
        } => render_input(frame, dialog_area, title, prompt, input, *cursor_pos),
        DialogKind::Confirm {
            title,
            message,
            details,
        } => render_confirm(frame, dialog_area, title, message, details),
        DialogKind::Message { title, message } => {
            render_message(frame, dialog_area, title, message, false)
        }
        DialogKind::Error { title, message } => {
            render_message(frame, dialog_area, title, message, true)
        }
    }
}

/// Helper to create a centered rectangle within `area`.
pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
