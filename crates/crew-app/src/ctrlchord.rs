//! The modifiers crew's arrows take (`winkeys` has the rest of the keys off a
//! Mac). Windows keeps Win+Arrow for snapping
//! windows, so off a Mac the arrows walk the grid with Alt, as Windows
//! Terminal walks its panes, and step the screens with Ctrl+Alt.
use winit::keyboard::{Key, ModifiersState};

use crate::app::CrewApp;
use crate::pane::PaneContent;
use crate::winkeys::WinChord;

impl CrewApp {
    /// The quit or walk chord a key press makes ([`crate::keychord::special`]).
    pub(crate) fn special_chord(
        &self,
        event: &winit::event::KeyEvent,
        mods: ModifiersState,
    ) -> Option<WinChord> {
        use winit::keyboard::PhysicalKey;
        use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
        let code = match event.physical_key {
            PhysicalKey::Code(c) => Some(c),
            PhysicalKey::Unidentified(_) => None,
        };
        let bare = event.key_without_modifiers();
        let mac = cfg!(target_os = "macos");
        crate::keychord::special(&event.logical_key, &bare, code, mods, mac)
    }

    /// Do what a [`Self::special_chord`] names. `true` when crew should quit.
    pub(crate) fn run_special(&mut self, chord: WinChord) -> bool {
        match chord {
            WinChord::Quit => return self.confirm_quit(),
            WinChord::Theme => self.toggle_theme(),
            WinChord::Gradient => self.cycle_gradient(),
            WinChord::Focus => self.toggle_focus_mode(),
            WinChord::MdSource => {
                if let Some(PaneContent::Chat(c)) =
                    self.panes.get_mut(self.focused).map(|p| &mut p.content)
                {
                    c.show_source = !c.show_source;
                }
                self.redraw();
            }
            _ => {}
        }
        false
    }

    /// An arrow with the modifiers that make it crew's: Cmd+Opt+←/→ steps
    /// the screens ([`crate::panezoom`]), Cmd+Opt+↑/↓ steps a terminal's
    /// commands ([`crate::cmdjump`]), Cmd+Arrow walks focus across the
    /// grid the way the eye does, and Cmd+Shift+Arrow carries the pane with
    /// it ([`crate::panedir`]). `true` when the key was one of them.
    pub(crate) fn arrow_chord(&mut self, key: &Key, mods: ModifiersState) -> bool {
        use crate::panedir::Dir;
        let mac = cfg!(target_os = "macos");
        let Some(dir) = crate::keychord::arrow_dir(key) else {
            return false;
        };
        let step = match dir {
            Dir::Left => -1,
            Dir::Right => 1,
            Dir::Up | Dir::Down => 0,
        };
        // Up/down on the screens' chord belongs to a focused terminal; on
        // any other pane it keeps the meaning it had before.
        let updown = step == 0 && screen_held(mods, mac);
        if step != 0 && screen_held(mods, mac) {
            self.step_screen(step);
        } else if updown && self.jump_command(dir == Dir::Up) {
            return true;
        } else if !grid_held(mods, mac) {
            return false;
        } else if mods.shift_key() {
            self.move_direction(dir);
        } else {
            self.focus_direction(dir);
        }
        true
    }
}

/// Whether the grid's arrows are held: Cmd, and off a Mac Alt (not AltGr).
pub(crate) fn grid_held(mods: ModifiersState, mac: bool) -> bool {
    mods.super_key() || (!mac && mods.alt_key() && !mods.control_key())
}

/// Whether the screens' arrows are held: Cmd+Opt on a Mac, Ctrl+Alt off it —
/// the chord a browser steps its tabs with. Not Ctrl+←/→, which a Mac keeps
/// for its own Spaces.
pub(crate) fn screen_held(mods: ModifiersState, mac: bool) -> bool {
    match mac {
        true => mods.super_key() && mods.alt_key(),
        false => mods.control_key() && mods.alt_key() && !mods.super_key(),
    }
}
