//! Off a Mac, the keys that are not letters (docs/plan/window-keys.md §3):
//! Ctrl+PgUp/PgDn step tabs, Ctrl+1…9 go to tab N, Alt+Home/End step the
//! screens, Alt+PgUp/PgDn jump between a terminal's commands, F11 maximizes.
//! The table is `winkeys::chord`; this picks the navigation rows out of it
//! and does them. A Mac has none of these (its Cmd+{ / Cmd+Opt chords go
//! through `chords.rs` and `ctrlchord`), so Ctrl+PgUp there reaches the pane.
use winit::keyboard::{Key, KeyCode, ModifiersState};

use crate::app::CrewApp;
use crate::winkeys::WinChord;

/// The navigation chord a key press makes, `None` on a Mac and for every
/// key crew does not claim (those reach the pane, modifiers encoded).
pub(crate) fn nav_chord(
    unmodified: &Key,
    code: Option<KeyCode>,
    mods: ModifiersState,
    mac: bool,
) -> Option<WinChord> {
    if mac {
        return None;
    }
    crate::winkeys::chord(unmodified, code, mods).filter(|c| {
        matches!(
            c,
            WinChord::Tab(_)
                | WinChord::TabN(_)
                | WinChord::Screen(_)
                | WinChord::CmdJump(_)
                | WinChord::Maximize
        )
    })
}

impl CrewApp {
    /// Do the navigation chord a pressed key makes. `true` when it was
    /// spent; an Alt+PgUp/PgDn over a pane that is no terminal is not, so
    /// the pane gets it.
    pub(crate) fn win_nav_key(
        &mut self,
        event: &winit::event::KeyEvent,
        mods: ModifiersState,
    ) -> bool {
        use winit::keyboard::PhysicalKey;
        use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
        let code = match event.physical_key {
            PhysicalKey::Code(c) => Some(c),
            PhysicalKey::Unidentified(_) => None,
        };
        let bare = event.key_without_modifiers();
        let Some(chord) = nav_chord(&bare, code, mods, cfg!(target_os = "macos")) else {
            return false;
        };
        match chord {
            WinChord::Tab(d) => self.step_tab(d),
            WinChord::TabN(n) => self.goto_tab(n),
            WinChord::Screen(d) => self.step_screen(d),
            WinChord::CmdJump(up) => return self.jump_command(up),
            _ => self.toggle_maximize(),
        }
        true
    }
}

#[cfg(test)]
#[path = "winnav_tests.rs"]
mod tests;
