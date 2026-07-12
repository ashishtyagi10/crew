//! Keyboard event dispatch for CrewApp.
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::session::key_to_bytes;
use crate::settingspane::SettingsAction;

impl CrewApp {
    /// Dispatch a single `KeyEvent` from `window_event`.
    pub(crate) fn on_key_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &winit::event::KeyEvent,
    ) {
        let mstate = self.mods.state();

        // The help overlay swallows the next key press to dismiss itself.
        if self.help_open && event.state.is_pressed() {
            self.help_open = false;
            self.redraw();
            return;
        }

        // Shift+PageUp/Down scroll a page; Shift+Home/End jump to top/bottom.
        if event.state.is_pressed() && mstate.shift_key() {
            match &event.logical_key {
                Key::Named(NamedKey::PageUp) => {
                    self.scroll_focused_page(true);
                    return;
                }
                Key::Named(NamedKey::PageDown) => {
                    self.scroll_focused_page(false);
                    return;
                }
                Key::Named(NamedKey::Home) => {
                    self.scroll_focused_end(true);
                    return;
                }
                Key::Named(NamedKey::End) => {
                    self.scroll_focused_end(false);
                    return;
                }
                _ => {}
            }
        }

        // Cmd+Q / Ctrl+Q quits — but with panes open, the first press only arms a
        // confirmation so a stray keystroke can't kill running shells/agents.
        if event.state.is_pressed()
            && (mstate.super_key() || mstate.control_key())
            && matches!(&event.logical_key, Key::Character(s) if s.as_str() == "q")
        {
            if self.confirm_quit() {
                event_loop.exit();
            }
            return;
        }

        // Ctrl+Tab / Ctrl+Shift+Tab cycle panes — works even over a focused
        // terminal (plain Tab still reaches the shell for completion).
        if event.state.is_pressed()
            && mstate.control_key()
            && matches!(&event.logical_key, Key::Named(NamedKey::Tab))
        {
            if !self.panes.is_empty() {
                let n = self.panes.len();
                self.input.focused = false;
                self.focused = if mstate.shift_key() {
                    (self.focused + n - 1) % n
                } else {
                    (self.focused + 1) % n
                };
            }
            self.redraw();
            return;
        }

        // Ctrl+Shift+L cycles themes (fixed presets, then rotation modes).
        if event.state.is_pressed()
            && mstate.control_key()
            && mstate.shift_key()
            && matches!(&event.logical_key, Key::Character(s) if s.eq_ignore_ascii_case("l"))
        {
            self.toggle_theme();
            return;
        }

        // Ctrl+Shift+M toggles markdown source view on the focused chat pane.
        if event.state.is_pressed()
            && mstate.control_key()
            && mstate.shift_key()
            && matches!(&event.logical_key, Key::Character(s) if s.eq_ignore_ascii_case("m"))
        {
            if let Some(pane) = self.panes.get_mut(self.focused) {
                if let PaneContent::Chat(c) = &mut pane.content {
                    c.show_source = !c.show_source;
                }
            }
            self.redraw();
            return;
        }

        // Super-chords (e.g. Cmd+I, Cmd+T, …) are handled first.
        if mstate.super_key() && event.state.is_pressed() {
            if let Key::Character(s) = &event.logical_key {
                let s = s.to_string();
                if self.handle_super_chord(&s) {
                    event_loop.exit();
                }
            }
            self.redraw();
            return;
        }

        // Alt+S saves a focused settings form (physical key: macOS Option+S
        // produces 'ß' as the logical key). Other panes see Alt+S as normal.
        if event.state.is_pressed()
            && mstate.alt_key()
            && matches!(event.physical_key, PhysicalKey::Code(KeyCode::KeyS))
            && self.save_focused_settings()
        {
            self.redraw();
            return;
        }

        // When the input bar is focused, all non-super keys go to it.
        if self.input.focused {
            if event.state.is_pressed()
                && matches!(&event.logical_key, Key::Named(NamedKey::Escape))
            {
                self.input.focused = false;
                self.redraw();
                return;
            }
            let submitted = self.input.on_key(event, mstate.control_key());
            if let Some(line) = submitted {
                if self.submit_input(line) {
                    event_loop.exit();
                    return;
                }
                crate::history::save(&self.input.history);
            }
            self.redraw();
            return;
        }

        // Route non-super keys to the focused pane.
        let focused = self.focused;
        let shift = mstate.shift_key();
        let mut settings_action: Option<SettingsAction> = None;
        let mut far_action: Option<crate::farpane::FarAction> = None;
        let mut chat_action: Option<crate::chatkeys::ChatAction> = None;
        let mut md_action: Option<crate::mdkeys::MdAction> = None;
        let mut is_terminal = false;
        if let Some(pane) = self.panes.get_mut(focused) {
            match &mut pane.content {
                // Terminal input is written below (so broadcast can reach all panes).
                PaneContent::Terminal(_) => is_terminal = true,
                PaneContent::Chat(c) => chat_action = c.on_key(event, &self.cwd),
                PaneContent::Settings(s) => {
                    settings_action = s.on_key(event, shift);
                }
                PaneContent::Far(f) => {
                    far_action = f.on_key(event);
                }
                // The swarm view is non-interactive; it ignores key input.
                PaneContent::Swarm(_) => {}
                PaneContent::Markdown(m) => {
                    md_action =
                        m.on_key(event, pane.grid.cols, pane.grid.rows, mstate.control_key())
                }
            }
        }
        if let Some(action) = far_action {
            use crate::farpane::FarAction;
            match action {
                FarAction::Close => {
                    self.close_pane(focused);
                }
                FarAction::Help => self.help_open = true,
                FarAction::Open(path) => {
                    let _ = open::that(path);
                }
                FarAction::Status(msg) => self.set_status(&msg),
            }
        }
        if let Some(action) = chat_action {
            use crate::chatkeys::ChatAction;
            match action {
                ChatAction::Close => {
                    self.close_pane(focused);
                }
            }
        }
        if let Some(action) = md_action {
            use crate::mdkeys::MdAction;
            match action {
                MdAction::Close => {
                    self.close_pane(focused);
                }
                MdAction::Status(msg) => self.set_status(msg),
            }
        }
        if is_terminal {
            if let Some(bytes) = key_to_bytes(event, mstate.control_key(), shift) {
                self.write_to_terminals(&bytes);
            }
        }
        if let Some(action) = settings_action {
            if let SettingsAction::Apply(cfg) = action {
                self.apply_settings(cfg);
            }
            // Save and Cancel both close the settings pane.
            self.close_pane(focused);
        }
        self.redraw();
    }

    /// Cmd+S / Alt+S: save-and-close when the focused pane is a settings
    /// form. Returns `false` when it isn't (the chord keeps its old meaning).
    pub(crate) fn save_focused_settings(&mut self) -> bool {
        let focused = self.focused;
        let Some(pane) = self.panes.get_mut(focused) else {
            return false;
        };
        let PaneContent::Settings(s) = &mut pane.content else {
            return false;
        };
        if let SettingsAction::Apply(cfg) = s.save() {
            self.apply_settings(cfg);
        }
        self.close_pane(focused);
        true
    }
}
