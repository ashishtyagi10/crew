//! `TermCore::input_modes` (child module of `model` — split for the 200-line
//! cap): the DEC private modes the app reads to route wheel, keys and focus.
use alacritty_terminal::term::TermMode;

use super::TermCore;

impl TermCore {
    /// Snapshot the DEC private modes that govern how input is routed.
    pub(crate) fn input_modes(&self) -> crate::modes::InputModes {
        let m = self.term.mode();
        let kitty = [
            TermMode::DISAMBIGUATE_ESC_CODES,
            TermMode::REPORT_EVENT_TYPES,
            TermMode::REPORT_ALTERNATE_KEYS,
            TermMode::REPORT_ALL_KEYS_AS_ESC,
            TermMode::REPORT_ASSOCIATED_TEXT,
        ]
        .iter()
        .enumerate()
        .fold(0u8, |f, (i, b)| f | (u8::from(m.contains(*b)) << i));
        crate::modes::InputModes {
            alt_screen: m.contains(TermMode::ALT_SCREEN),
            mouse: m.intersects(TermMode::MOUSE_MODE),
            mouse_drag: m.contains(TermMode::MOUSE_DRAG),
            mouse_motion: m.contains(TermMode::MOUSE_MOTION),
            sgr_mouse: m.contains(TermMode::SGR_MOUSE),
            pixel_mouse: self.pixel.on(),
            app_cursor: m.contains(TermMode::APP_CURSOR),
            alternate_scroll: m.contains(TermMode::ALTERNATE_SCROLL),
            focus_events: m.contains(TermMode::FOCUS_IN_OUT),
            kitty_keys: kitty != 0,
            kitty_flags: kitty,
        }
    }

    /// The program hid the cursor (DECTCEM off, `ESC [ ? 25 l`). Not an
    /// `InputModes` field: nothing routes input by it; only the leftover
    /// reset at a fresh prompt reads it (`promptmodes`).
    pub(crate) fn cursor_hidden(&self) -> bool {
        !self.term.mode().contains(TermMode::SHOW_CURSOR)
    }
}
