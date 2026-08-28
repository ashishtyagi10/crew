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

/// The accent exactly as the user set it — [`DEFAULT_ACCENT`] until
/// [`set_accent`] is called. For round-trips: the settings swatch, `/accent`'s
/// own report, config save. Everything that *draws* wants [`accent`].
pub fn raw_accent() -> (u8, u8, u8) {
    unpack(ACCENT.load(Ordering::Relaxed))
}

/// The accent as it is drawn: [`raw_accent`] floored against the page it will
/// land on.
///
/// Every theme's own `accent_default` already clears the floor — that is what
/// picking it *meant*. A user-set one does not have to, and the app's own
/// brand green (`#00ffa0`, and the value a user carries over from a dark
/// theme) reads at **1.2 against every light page in the set**: on paper-light
/// the whole left nav — section legends, the clock, the load, the PANES key,
/// the CPU trace — went to a mint that is not there. `spark`, `warn`,
/// `danger`, `cursor` and `link` each grew this floor of their own; the accent
/// is the one colour on the canvas that never did, and it is the one the user
/// can change.
///
/// Floored with [`crew_theme::readable::enforced`] rather than `against`,
/// because a saturated yellow cannot reach the floor at *any* lightness on a
/// cream page — it has to give up chroma, and giving up chroma is better than
/// giving up the floor for the one colour the app cannot re-pick.
///
/// Computed once per (accent, page) rather than per call: the walk is an oklch
/// search, and this is read a few dozen times a frame.
pub fn accent() -> (u8, u8, u8) {
    let (raw, page) = (raw_accent(), crew_theme::theme().page_bg);
    // The floor is part of the key, not just the computation: the OS's high-
    // contrast switch raises it without touching either the accent or the
    // theme, and a memo keyed only on those two answered the old floor for the
    // rest of the session — the accessibility request landing everywhere in
    // the palette except the one colour the user picked.
    let floor = crew_theme::contrast::text_floor();
    let key = (pack(raw), pack(page), floor.to_bits());
    MEMO.with(|m| {
        let (k_raw, k_page, k_floor, val) = m.get();
        if (k_raw, k_page, k_floor) == key {
            return unpack(val);
        }
        let fixed = crew_theme::readable::enforced(raw, page, floor);
        m.set((key.0, key.1, key.2, pack(fixed)));
        fixed
    })
}

/// How far the FOCUS accent must sit from `text_muted` — the colour it
/// replaces when a control takes focus.
///
/// [`accent`] is floored against the page, which says it can be read; it says
/// nothing about whether it can be told from the colour it is standing in for.
/// Measured across the set: sepia-dark **1.04** and crt-violet **1.06** —
/// accent and muted at the same lightness, so a focused input's border was
/// distinguishable by hue alone, and on a tube not at all.
const FOCUS_FLOOR: f32 = 1.6;

/// Stiffer where hue is not available. A single-phosphor screen has one hue
/// and lightness is the whole of the signal, so "different colour" has to
/// mean "different brightness" there or it means nothing.
const TUBE_FOCUS_FLOOR: f32 = 1.8;

/// The accent as a FOCUS marker: [`accent`], pushed until it clears
/// [`FOCUS_FLOOR`] against `text_muted`. A floor, not a restyle — most presets
/// clear it untouched and are handed their own accent back.
///
/// Use this wherever focus is drawn by swapping muted ink for accent ink (the
/// settings form's boxed inputs, its card legends and its buttons). Use plain
/// [`accent`] where the accent is the subject rather than a state.
pub fn focus_accent() -> (u8, u8, u8) {
    let t = crew_theme::theme();
    let floor = match t.is_tube() {
        true => TUBE_FOCUS_FLOOR,
        false => FOCUS_FLOOR,
    };
    crew_theme::readable::enforced(accent(), t.text_muted, floor)
}

/// [`focus_accent`] as a ratatui colour.
pub fn focus_color() -> ratatui::style::Color {
    let (r, g, b) = focus_accent();
    ratatui::style::Color::Rgb(r, g, b)
}

thread_local! {
    /// `(accent, page, floor bits, floored)`. The render path is one thread;
    /// a second one simply keeps its own.
    static MEMO: std::cell::Cell<(u32, u32, u32, u32)> =
        const { std::cell::Cell::new((u32::MAX, u32::MAX, u32::MAX, 0)) };
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
    fn set_then_raw_accent_round_trips() {
        // Serialise with any other test that reads the accent global.
        let _g = crate::palette::test_guard();
        set_accent((10, 20, 30));
        assert_eq!(raw_accent(), (10, 20, 30));
        set_accent(DEFAULT_ACCENT); // restore so other tests see the default
        assert_eq!(raw_accent(), DEFAULT_ACCENT);
    }

    /// The defect: crew's own brand green, and the value anyone carries over
    /// from a dark theme, reads at 1.2 against every light page in the set.
    /// Every theme's own default already clears the floor and must come back
    /// untouched — the floor is for the colour the *user* can set.
    #[test]
    fn a_user_accent_is_floored_against_the_page_it_lands_on() {
        let _a = crate::palette::test_guard();
        let _g = crate::app::theme_test_guard();
        let floor = crew_theme::contrast::text_floor();
        let mut bad: Vec<String> = Vec::new();
        for id in crew_theme::ALL_THEMES {
            crew_theme::set_theme(id);
            let page = crew_theme::theme().page_bg;

            set_accent(DEFAULT_ACCENT);
            let r = crew_theme::contrast_ratio(accent(), page);
            if r < floor {
                bad.push(format!("{}: crew green floored to {r:.2}", id.as_str()));
            }

            let d = crew_theme::theme().accent_default;
            set_accent(d);
            if accent() != d {
                bad.push(format!("{}: its own default was moved", id.as_str()));
            }
        }
        set_accent(DEFAULT_ACCENT);
        assert!(bad.is_empty(), "{}", bad.join("\n  "));
    }
    /// Focus in a form is drawn by swapping muted ink for accent ink, so the
    /// two have to be tellable apart. `accent()` is floored against the PAGE
    /// — it says the colour can be read, not that it can be told from the one
    /// it stands in for. Measured before this floor: sepia-dark **1.04** and
    /// crt-violet **1.06**, i.e. the same lightness, so the focused input's
    /// border differed by hue alone — and on a single-phosphor tube, not at
    /// all.
    #[test]
    fn the_focus_accent_can_be_told_from_the_ink_it_replaces() {
        let _a = crate::palette::test_guard();
        let _g = crate::app::theme_test_guard();
        let mut tubes = 0;
        for id in crew_theme::ALL_THEMES {
            crew_theme::set_theme(id);
            crate::palette::set_accent(crew_theme::theme().accent_default);
            let t = crew_theme::theme();
            let want = match t.is_tube() {
                true => {
                    tubes += 1;
                    super::TUBE_FOCUS_FLOOR
                }
                false => super::FOCUS_FLOOR,
            };
            let got = crew_theme::contrast_ratio(super::focus_accent(), t.text_muted);
            assert!(
                got >= want,
                "{}: focus accent vs muted = {got:.2} (want {want})",
                id.as_str(),
            );
            // …and it must still read on the page it is drawn on.
            let page = crew_theme::contrast_ratio(super::focus_accent(), t.page_bg);
            assert!(
                page >= crew_theme::contrast::mark_floor(),
                "{}: and it stays legible: {page:.2}",
                id.as_str(),
            );
        }
        assert_eq!(tubes, 4, "every tube was actually checked");
        crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    }
}
