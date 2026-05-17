//! Full-screen overlays (editor, viewer, diff) and the focused embedded
//! terminal. When one of these owns the screen, all key input is routed
//! to it; the only escape hatches are Tab/F4 (cycle), Ctrl+Left/Right
//! (jump to file panel), and Ctrl+W (close terminal).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use farx_core::{Action, PanelSide};

use crate::components::diff_view::DiffAction;
use crate::components::editor::EditorAction;
use crate::components::embedded_terminal::key_to_bytes;
use crate::components::viewer::ViewerAction;

use super::super::App;

impl App {
    pub(super) fn key_route_fullscreen(&mut self, key: KeyEvent) -> Option<Action> {
        if let Some(ref mut editor) = self.editor {
            match editor.handle_key_event(key) {
                EditorAction::Close | EditorAction::SaveAndClose => {
                    self.editor = None;
                    self.refresh_both_panels();
                }
                EditorAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut viewer) = self.viewer {
            match viewer.handle_key_event(key) {
                ViewerAction::Close => self.viewer = None,
                ViewerAction::None => {}
            }
            return Some(Action::Noop);
        }

        if let Some(ref mut diff) = self.diff_view {
            match diff.handle_key_event(key) {
                DiffAction::Close => self.diff_view = None,
                DiffAction::None => {}
            }
            return Some(Action::Noop);
        }

        None
    }

    pub(super) fn key_route_terminal(&mut self, key: KeyEvent) -> Option<Action> {
        let tid = self.focused_terminal?;

        if key.code == KeyCode::F(4)
            || (key.code == KeyCode::Tab && key.modifiers == KeyModifiers::NONE)
        {
            self.cycle_focus();
            return Some(Action::Noop);
        }
        if key.modifiers == KeyModifiers::CONTROL
            && (key.code == KeyCode::Left || key.code == KeyCode::Right)
        {
            self.focused_terminal = None;
            self.active_panel = if key.code == KeyCode::Left {
                PanelSide::Left
            } else {
                PanelSide::Right
            };
            return Some(Action::Noop);
        }
        if key.code == KeyCode::Char('w') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.close_terminal(tid);
            return Some(Action::Noop);
        }

        if let Some(bytes) = key_to_bytes(&key) {
            if let Some(term) = self.terminals.get_mut(tid) {
                if term.alive {
                    term.write_input(&bytes);
                    term.poll_output();
                } else {
                    self.close_terminal(tid);
                }
            }
        }
        Some(Action::Noop)
    }
}
