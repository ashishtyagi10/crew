//! The left-nav RESTART card: the button a finished auto-update leaves behind.
//!
//! Updates install themselves in the background (see [`crate::autoupdate`]) —
//! there is no command to run and nothing to wait for. What is left over is
//! the one step crew cannot take on the user's behalf: the restart that swaps
//! the running process for the binary already on disk. That used to be a line
//! of legend text saying `/update`, which is a command for something that has
//! already happened. It is a button now: it blinks until it is dealt with, and
//! the whole card is the target.
use crew_render::CellView;

use crate::updatecard::{glyph, write};

/// Accent↔dim on the blink clock, so the card keeps asking rather than asking
/// once. Steady accent when motion is off — the same rule every other
/// animation in the app follows, and the reason this takes `now_ms` instead
/// of reading the clock itself (the tests drive it).
pub(crate) fn button_fg(now_ms: u64) -> (u8, u8, u8) {
    if crate::motion::level() == crate::motion::MotionLevel::Off {
        return crate::palette::accent();
    }
    if (now_ms / crate::attention::BLINK_MS).is_multiple_of(2) {
        return crate::palette::accent();
    }
    crew_theme::theme().legend_off
}

/// Interior cells: the blinking call to action, then the version it lands on.
///
/// Row 1 is the *destination*, not a second instruction — a restart is cheap
/// to do and annoying to do for no reason, so the card says what it buys.
pub(crate) fn restart_cells(new_version: &str, now_ms: u64, cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 4 || rows == 0 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let max = cols.saturating_sub(1);
    let mut out = vec![glyph(0, 0, '\u{27f3}', button_fg(now_ms), t.page_bg)];
    write(&mut out, "restart", 2, 0, button_fg(now_ms), max, t.page_bg);
    if rows > 1 {
        let to = format!("\u{2192} v{new_version}");
        write(&mut out, &to, 2, 1, t.ink, max, t.page_bg);
    }
    out
}

#[cfg(test)]
#[path = "restartcard_tests.rs"]
mod tests;
