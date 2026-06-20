//! Key-event routing. The top-level `handle_key_event` walks a priority
//! chain of overlay/modal handlers; the first one to claim the key returns
//! `Some(Action)` and execution stops. Anything that falls through is
//! resolved by the panel keymap.

mod fullscreen;
mod input_modals;
mod modals;
mod overlays;
mod text_input;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use farx_core::Action;

use super::App;

impl App {
    /// Top-level key dispatcher: routes through the active overlay stack
    /// in priority order, falling through to the panel keymap if no
    /// overlay claims the key.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if let Some(a) = self.key_route_fullscreen(key) {
            return a;
        }
        // Global: F1 (or Alt+Enter) focuses the main command input from
        // anywhere, including while an agent panel owns the keyboard.
        if (key.code == KeyCode::F(1) && key.modifiers == KeyModifiers::NONE)
            || (key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::ALT))
        {
            self.focused_terminal = None;
            return Action::Noop;
        }
        // Global: F2 cycles focus across agent panels (Tab now belongs to the
        // focused agent).
        if key.code == KeyCode::F(2) && key.modifiers == KeyModifiers::NONE {
            self.cycle_focus();
            return Action::Noop;
        }
        if let Some(a) = self.key_route_terminal(key) {
            return a;
        }
        if let Some(a) = self.key_route_feedback_help_update(key) {
            return a;
        }
        if let Some(a) = self.key_route_overlays(key) {
            return a;
        }
        if let Some(a) = self.key_route_modals(key) {
            return a;
        }
        if let Some(a) = self.key_route_filter(key) {
            return a;
        }
        if let Some(a) = self.key_route_command_line(key) {
            return a;
        }
        self.keymap.resolve_panel(&key)
    }
}
