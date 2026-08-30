//! Keyboard event dispatch for CrewApp.
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

use crate::app::CrewApp;
use crate::keychord::{arrow_dir, is_compact_chord};
use crate::pane::PaneContent;

impl CrewApp {
    /// Dispatch a single `KeyEvent` from `window_event`.
    pub(crate) fn on_key_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: &winit::event::KeyEvent,
    ) {
        let mstate = self.mods.state();
        // The help overlay: arrows and page keys walk its list, anything else
        // dismisses it. It is longer than a window and always has been — it
        // used to answer that by silently cutting the bottom off.
        //
        // A Cmd chord is a command, not a keystroke for the filter box. Every
        // one of them used to be typed into the search instead: with the
        // overlay up, Cmd+T put a "t" in the filter and opened no shell. A
        // held super key closes the overlay and lets the chord through —
        // except Cmd+/, which IS this overlay, and so simply puts it away.
        if self.help_open && event.state.is_pressed() {
            if mstate.super_key() {
                self.close_help();
                let toggled = matches!(&event.logical_key,
                    Key::Character(s) if s.as_str() == "/" || s.as_str() == "?");
                if toggled {
                    self.redraw();
                    return;
                }
            } else {
                self.help_key(&event.logical_key);
                self.redraw();
                return;
            }
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

        // Ctrl+Shift+G steps the canvas gradient — the colour's answer to
        // Ctrl+Shift+L above, and the walk passes through the theme's own
        // gradient once a lap so the key can always get you back.
        if event.state.is_pressed()
            && mstate.control_key()
            && mstate.shift_key()
            && matches!(&event.logical_key, Key::Character(s) if s.eq_ignore_ascii_case("g"))
        {
            self.cycle_gradient();
            return;
        }

        // Ctrl+Shift+F enters/leaves focus mode — the third of the Ctrl+Shift
        // walks, and the one that changes what crew is allowed to do rather
        // than what it looks like.
        if event.state.is_pressed()
            && mstate.control_key()
            && mstate.shift_key()
            && matches!(&event.logical_key, Key::Character(s) if s.eq_ignore_ascii_case("f"))
        {
            self.toggle_focus_mode();
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

        // Ctrl+O toggles the compact transcript view on the focused chat
        // pane — same global reach as Ctrl+Shift+M above (fires even with
        // the input bar focused). Unlike Ctrl+Shift+M it only consumes the
        // key when the focused pane actually IS a chat pane; otherwise it
        // falls through so terminals still get the raw 0x0f byte.
        if event.state.is_pressed()
            && is_compact_chord(&event.logical_key, mstate)
            && self.toggle_compact_focused()
        {
            self.redraw();
            return;
        }

        // Cmd+Arrow walks focus across the tiled grid the way the eye does,
        // and Cmd+Shift+Arrow carries the pane with it (see `panedir`). Both
        // sit above the Character super-chords because an arrow arrives as a
        // NamedKey, and above the input-bar early-return so the grid stays
        // navigable while the composer holds the keys.
        if event.state.is_pressed() && mstate.super_key() {
            if let Some(dir) = arrow_dir(&event.logical_key) {
                if mstate.shift_key() {
                    self.move_direction(dir);
                } else {
                    self.focus_direction(dir);
                }
                self.redraw();
                return;
            }
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

        self.route_key_to_focused(event, mstate);
        self.redraw();
    }
}
