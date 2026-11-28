//! Keyboard event dispatch for CrewApp.
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

use crate::app::CrewApp;
use crate::keychord::is_compact_chord;

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

        // Shift+PageUp/Down page and Shift+Home/End jump the scrollback
        // (`shiftscroll`), or reach a full-screen program as its own keys.
        if self.view_edit_key(event, mstate) || self.shift_scroll_key(event, mstate) {
            return;
        }

        // Quit, and the theme / gradient / focus / markdown-source walks
        // (`keychord::special`: Cmd+Q and Ctrl+Shift+L/G/F/M on a Mac,
        // Alt+Q/L/B/H/Y off one). Quit with panes open only arms a
        // confirmation, so a stray keystroke can't kill running shells.
        if event.state.is_pressed() {
            if let Some(c) = self.special_chord(event, mstate) {
                if self.run_special(c) {
                    event_loop.exit();
                }
                return;
            }
        }

        // Ctrl+Tab / Ctrl+Shift+Tab cycle panes — works even over a focused
        // terminal (plain Tab still reaches the shell for completion).
        if event.state.is_pressed()
            && mstate.control_key()
            && matches!(&event.logical_key, Key::Named(NamedKey::Tab))
        {
            self.cycle_tab_panes(if mstate.shift_key() { -1 } else { 1 });
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

        // Cmd+Arrow walks the grid, Cmd+Opt+←/→ steps the screens
        // (`arrow_chord`). Above the Character super-chords because an arrow
        // arrives as a NamedKey, and above the input-bar early-return so the
        // grid stays navigable while the composer holds the keys.
        if event.state.is_pressed() && self.arrow_chord(&event.logical_key, mstate) {
            self.redraw();
            return;
        }

        // Cmd+Opt+1…9 (Ctrl+Alt off a Mac) brings tab N on: read off the
        // physical key, since Opt turns the digit into another character.
        if let Some(n) = crate::tablines::tab_digit(event, mstate) {
            self.goto_tab(n);
            self.redraw();
            return;
        }

        // Super-chords (Cmd+I, Cmd+T, …; Alt+I, Alt+T … off a Mac) first.
        if let Some(s) = crate::keychord::cmd_chord(event, mstate) {
            if self.handle_super_chord(&s) {
                event_loop.exit();
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

        // Hint mode takes every key while it is on: the letters ARE the
        // interface, exactly as the `/find` bar's are.
        if self.hint_key(event) || self.nav_key(event) {
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
