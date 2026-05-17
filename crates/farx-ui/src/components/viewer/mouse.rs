use crossterm::event::{MouseEvent, MouseEventKind};

use super::state::ViewerState;
use super::ViewerAction;

impl ViewerState {
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) -> ViewerAction {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.scroll_up(3);
                ViewerAction::None
            }
            MouseEventKind::ScrollDown => {
                self.scroll_down(3);
                ViewerAction::None
            }
            _ => ViewerAction::None,
        }
    }
}
