//! Where crew's Cmd chords land on Windows and Linux (`keychord::stand_in`):
//! whether a plain Ctrl key here belongs to a terminal or a line being typed,
//! and the modifiers the arrows take. Windows keeps Win+Arrow for snapping
//! windows, so off a Mac the arrows walk the grid with Alt, as Windows
//! Terminal walks its panes, and step the screens with Ctrl+Alt.
use winit::keyboard::{Key, ModifiersState};

use crate::app::CrewApp;
use crate::pane::PaneContent;

impl CrewApp {
    /// Whether a plain Ctrl key here goes to a program or a typed line: the
    /// input bar, a terminal, or a pane with a line to type into.
    pub(crate) fn typing_here(&self) -> bool {
        let typed = |p: &crate::pane::Pane| {
            matches!(
                p.content,
                PaneContent::Terminal(_)
                    | PaneContent::Chat(_)
                    | PaneContent::Todo(_)
                    | PaneContent::Settings(_)
                    | PaneContent::Far(_)
            ) || matches!(&p.content, PaneContent::Inbox(i) if i.composing())
        };
        self.input.focused || self.panes.get(self.focused).is_some_and(typed)
    }

    /// The Cmd chord a key press makes here ([`crate::keychord::cmd_chord`]).
    pub(crate) fn chord_here(
        &self,
        event: &winit::event::KeyEvent,
        mods: ModifiersState,
    ) -> Option<String> {
        crate::keychord::cmd_chord(event, mods, self.typing_here())
    }

    /// An arrow with the modifiers that make it crew's: Cmd+Opt+←/→ steps
    /// the screens ([`crate::panezoom`]), Cmd+Arrow walks focus across the
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
        if step != 0 && screen_held(mods, mac) {
            self.step_screen(step);
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
