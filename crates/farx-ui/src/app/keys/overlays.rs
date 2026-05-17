//! Top-priority overlays: feedback prompts (delete confirmations, etc.),
//! the help screen, and the update modal (Confirm / Done / Failed).

use crossterm::event::KeyEvent;
use farx_core::Action;

use crate::components::feedback::FeedbackResult;
use crate::components::update_modal::{UpdateAction, UpdateState};

use super::super::App;

impl App {
    pub(super) fn key_route_feedback_help_update(&mut self, key: KeyEvent) -> Option<Action> {
        match self.feedback.handle_key(key) {
            FeedbackResult::Confirmed(_) => {
                if let Some(action) = self.feedback.take_confirm() {
                    self.execute_confirm(action);
                }
                return Some(Action::Noop);
            }
            FeedbackResult::Rejected | FeedbackResult::Consumed => return Some(Action::Noop),
            FeedbackResult::NotHandled => {}
        }

        if let Some(ref mut help) = self.help {
            help.handle_key_event(key);
            if !help.active {
                self.help = None;
            }
            return Some(Action::Noop);
        }

        if let Some(ref state) = self.update_state {
            if state.is_modal() {
                match state.handle_key_event(key) {
                    UpdateAction::Confirmed => {
                        if let Some(UpdateState::Confirm { latest, .. }) = self.update_state.take()
                        {
                            self.update_state = Some(UpdateState::Installing { latest });
                            self.pending_install = true;
                        }
                    }
                    UpdateAction::Cancelled | UpdateAction::Dismissed => {
                        self.update_state = None;
                    }
                    UpdateAction::None => {}
                }
                return Some(Action::Noop);
            }
        }

        None
    }
}
