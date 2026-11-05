//! `TermCore::input_modes` (child module of `model` — split for the 200-line
//! cap): the DEC private modes the app reads to route wheel, keys and focus.
use alacritty_terminal::term::TermMode;

use super::TermCore;

impl TermCore {
    /// Snapshot the DEC private modes that govern how input is routed.
    pub(crate) fn input_modes(&self) -> crate::modes::InputModes {
        let m = self.term.mode();
        crate::modes::InputModes {
            alt_screen: m.contains(TermMode::ALT_SCREEN),
            mouse: m.intersects(TermMode::MOUSE_MODE),
            mouse_drag: m.contains(TermMode::MOUSE_DRAG),
            mouse_motion: m.contains(TermMode::MOUSE_MOTION),
            sgr_mouse: m.contains(TermMode::SGR_MOUSE),
            app_cursor: m.contains(TermMode::APP_CURSOR),
            alternate_scroll: m.contains(TermMode::ALTERNATE_SCROLL),
            focus_events: m.contains(TermMode::FOCUS_IN_OUT),
        }
    }
}
