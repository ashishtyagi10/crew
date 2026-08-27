//! The app's themeable colour palette. Render code is a web of free functions
//! that never see the config, so the one user-tunable colour — the **accent**
//! (Crew green by default) — lives here behind a lock-free global set once at
//! startup (and re-set by settings / theme switches). `accent()` returns the built-in
//! default until `set_accent` is called, so tests and headless paths are
//! unaffected.
use std::sync::atomic::{AtomicU32, Ordering};

/// The built-in accent: Crew green.
pub const DEFAULT_ACCENT: (u8, u8, u8) = (0, 255, 160);

const fn pack((r, g, b): (u8, u8, u8)) -> u32 {
    (r as u32) << 16 | (g as u32) << 8 | b as u32
}

fn unpack(v: u32) -> (u8, u8, u8) {
    ((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

/// Packed accent RGB, initialised to [`DEFAULT_ACCENT`].
///
/// The initialiser used to be a hand-written `(g << 8) | b` — the red channel
/// simply missing, which was correct only because the default's red happens
/// to be zero, and would have silently shipped the wrong startup accent for
/// any other default. `pack` is a `const fn` so the initialiser is the same
/// expression every other packing goes through and the class of bug is gone
/// rather than guarded.
static ACCENT: AtomicU32 = AtomicU32::new(pack(DEFAULT_ACCENT));

/// …and the default itself is what it says it is. A const assertion, because
/// the test that used to check this read the LIVE global: any test that had
/// called `set_accent` first made it fail, which is a race, not a finding.
const _: () = assert!(pack(DEFAULT_ACCENT) == 0x00_FF_A0);

/// Set the active accent colour (called from config at startup / on settings save).
pub fn set_accent(rgb: (u8, u8, u8)) {
    ACCENT.store(pack(rgb), Ordering::Relaxed);
}

/// The active accent colour — [`DEFAULT_ACCENT`] until [`set_accent`] is called.
pub fn accent() -> (u8, u8, u8) {
    unpack(ACCENT.load(Ordering::Relaxed))
}

/// The active accent as a ratatui [`Color`](ratatui::style::Color), for the
/// overlay widgets (help / command menu / settings / far) drawn with ratatui.
pub fn accent_color() -> ratatui::style::Color {
    let (r, g, b) = accent();
    ratatui::style::Color::Rgb(r, g, b)
}

/// Parse a `#rrggbb` / `rrggbb` hex string into an RGB triple. Returns `None`
/// for anything that isn't exactly six hex digits (optionally `#`-prefixed).
pub fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Serialises tests that read or mutate the accent global. Any test that calls
/// [`set_accent`] — or asserts against [`accent`]/[`accent_color`] — should hold
/// this guard so the process-wide value isn't changed mid-assertion by a
/// concurrently-running test.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_round_trips() {
        for rgb in [(0, 255, 160), (255, 255, 255), (0, 0, 0), (18, 200, 7)] {
            assert_eq!(unpack(pack(rgb)), rgb);
        }
    }

    #[test]
    fn parse_hex_accepts_with_and_without_hash() {
        assert_eq!(parse_hex("#00ffa0"), Some((0, 255, 160)));
        assert_eq!(parse_hex("00FFA0"), Some((0, 255, 160)));
        assert_eq!(parse_hex("  #123456 "), Some((0x12, 0x34, 0x56)));
    }

    #[test]
    fn parse_hex_rejects_bad_input() {
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("#fff"), None); // shorthand not supported
        assert_eq!(parse_hex("#gggggg"), None);
        assert_eq!(parse_hex("0xffaa00"), None);
    }

    #[test]
    fn set_then_accent_round_trips() {
        // Serialise with any other test that reads the accent global.
        let _g = crate::palette::test_guard();
        set_accent((10, 20, 30));
        assert_eq!(accent(), (10, 20, 30));
        assert_eq!(accent_color(), ratatui::style::Color::Rgb(10, 20, 30));
        set_accent(DEFAULT_ACCENT); // restore so other tests see the default
        assert_eq!(accent(), DEFAULT_ACCENT);
    }
}
