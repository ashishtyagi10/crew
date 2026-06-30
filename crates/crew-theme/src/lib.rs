//! Crew's color themes. A single `Theme` struct holds every UI colour; two
//! `&'static` presets (`PAPER_DARK`, `PAPER_LIGHT`) give crew an e-ink-reader
//! look. The active theme lives behind a lock-free `AtomicU8` so the winit
//! render thread can read it every frame without blocking. No dependencies and
//! no knowledge of the other crates — they import this one.
use std::sync::atomic::{AtomicU8, Ordering};

/// Every colour the UI draws with. RGB triples; `ansi` is the 16-slot terminal
/// palette (indices 0–15) used for shell output.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// Window/pane background — also the wgpu clear colour and the terminal's
    /// default background, so cells at the default bg show the page through.
    pub page_bg: (u8, u8, u8),
    /// Primary chrome text ("ink").
    pub ink: (u8, u8, u8),
    /// Secondary/body text (slightly softer than `ink`).
    pub text_muted: (u8, u8, u8),
    /// Terminal default foreground / background for unstyled shell output.
    pub term_fg: (u8, u8, u8),
    pub term_bg: (u8, u8, u8),
    /// Unfocused / focused rounded pane border.
    pub border_normal: (u8, u8, u8),
    pub border_focused: (u8, u8, u8),
    /// Legend text on an unfocused pane card.
    pub legend_off: (u8, u8, u8),
    /// Default accent when the user hasn't set one in config.
    pub accent_default: (u8, u8, u8),
    /// Status line / scroll hint amber.
    pub status_fg: (u8, u8, u8),
    /// Broadcast indicator.
    pub broadcast: (u8, u8, u8),
    /// Pane activity dot.
    pub activity: (u8, u8, u8),
    /// Bell indicator.
    pub bell: (u8, u8, u8),
    /// Dim hint text on the input bar.
    pub dim: (u8, u8, u8),
    /// Input placeholder text.
    pub placeholder: (u8, u8, u8),
    /// Hint text (chat layout).
    pub hint_fg: (u8, u8, u8),
    /// Search-highlight background.
    pub find_hl_bg: (u8, u8, u8),
    /// 16-colour ANSI palette for shell output (muted "ink" tones).
    pub ansi: [(u8, u8, u8); 16],
}

/// Warm e-ink "night" page — charcoal-brown, never blue-black. The default.
pub static PAPER_DARK: Theme = Theme {
    page_bg: (32, 32, 28),
    ink: (207, 199, 184),
    text_muted: (170, 162, 148),
    term_fg: (207, 199, 184),
    term_bg: (32, 32, 28),
    border_normal: (74, 70, 61),
    border_focused: (138, 132, 116),
    legend_off: (107, 101, 87),
    accent_default: (199, 154, 94),
    status_fg: (204, 170, 106),
    broadcast: (181, 138, 168),
    activity: (125, 154, 184),
    bell: (204, 170, 106),
    dim: (110, 104, 92),
    placeholder: (95, 90, 80),
    hint_fg: (107, 101, 87),
    find_hl_bg: (74, 67, 31),
    ansi: [
        (50, 49, 44),    // 0  black
        (192, 106, 90),  // 1  red
        (154, 167, 106), // 2  green
        (204, 170, 106), // 3  yellow
        (125, 154, 184), // 4  blue
        (181, 138, 168), // 5  magenta
        (127, 176, 170), // 6  cyan
        (207, 199, 184), // 7  white
        (107, 101, 87),  // 8  bright black
        (214, 128, 112), // 9  bright red
        (176, 189, 128), // 10 bright green
        (224, 192, 128), // 11 bright yellow
        (147, 176, 206), // 12 bright blue
        (203, 160, 190), // 13 bright magenta
        (149, 198, 192), // 14 bright cyan
        (236, 229, 214), // 15 bright white
    ],
};

/// Warm paper "day" page — soft off-white with ink-toned output.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (244, 241, 234),
    ink: (43, 40, 37),
    text_muted: (90, 84, 75),
    term_fg: (43, 40, 37),
    term_bg: (244, 241, 234),
    border_normal: (201, 194, 178),
    border_focused: (140, 132, 117),
    legend_off: (168, 159, 141),
    accent_default: (156, 107, 63),
    status_fg: (150, 110, 40),
    broadcast: (150, 70, 120),
    activity: (60, 100, 140),
    bell: (160, 120, 40),
    dim: (140, 132, 118),
    placeholder: (160, 152, 138),
    hint_fg: (160, 152, 138),
    find_hl_bg: (232, 220, 168),
    ansi: [
        (43, 40, 37),    // 0  black
        (156, 59, 46),   // 1  red (brick)
        (93, 107, 58),   // 2  green (sage)
        (154, 123, 46),  // 3  yellow (ochre)
        (63, 90, 120),   // 4  blue (faded indigo)
        (125, 75, 110),  // 5  magenta (mauve)
        (63, 111, 107),  // 6  cyan (teal)
        (92, 86, 75),    // 7  white (warm gray)
        (120, 113, 99),  // 8  bright black
        (178, 82, 66),   // 9  bright red
        (122, 134, 82),  // 10 bright green
        (180, 148, 74),  // 11 bright yellow
        (88, 116, 148),  // 12 bright blue
        (150, 100, 135), // 13 bright magenta
        (88, 140, 134),  // 14 bright cyan
        (60, 56, 50),    // 15 bright white (boldest ink)
    ],
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    PaperDark,
    PaperLight,
}

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "paper-dark",
            ThemeId::PaperLight => "paper-light",
        }
    }

    pub fn from_str(s: &str) -> Option<ThemeId> {
        match s.trim() {
            "paper-dark" => Some(ThemeId::PaperDark),
            "paper-light" => Some(ThemeId::PaperLight),
            _ => None,
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::PaperDark => &PAPER_DARK,
            ThemeId::PaperLight => &PAPER_LIGHT,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            ThemeId::PaperDark => 0,
            ThemeId::PaperLight => 1,
        }
    }

    fn from_u8(v: u8) -> ThemeId {
        match v {
            1 => ThemeId::PaperLight,
            _ => ThemeId::PaperDark,
        }
    }
}

/// Active theme id, default `PaperDark` (0). Lock-free for per-frame reads.
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// Set the active theme (startup, `/theme`, hotkey).
pub fn set_theme(id: ThemeId) {
    CURRENT.store(id.as_u8(), Ordering::Relaxed);
}

/// The active theme id.
pub fn current_id() -> ThemeId {
    ThemeId::from_u8(CURRENT.load(Ordering::Relaxed))
}

/// The active theme. Read every frame on the winit thread — lock-free.
pub fn theme() -> &'static Theme {
    current_id().theme()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialises tests that mutate the process-wide CURRENT.
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn default_is_paper_dark() {
        let _g = guard();
        // At rest (no set_theme yet in this process) the default id is PaperDark.
        // We don't assert on a possibly-mutated global; just the mapping.
        assert_eq!(ThemeId::from_u8(0), ThemeId::PaperDark);
    }

    #[test]
    fn id_string_round_trip() {
        for id in [ThemeId::PaperDark, ThemeId::PaperLight] {
            assert_eq!(ThemeId::from_str(id.as_str()), Some(id));
        }
        assert_eq!(ThemeId::from_str("nope"), None);
        assert_eq!(
            ThemeId::from_str("  paper-light "),
            Some(ThemeId::PaperLight)
        );
    }

    #[test]
    fn set_then_current_round_trips() {
        let _g = guard();
        set_theme(ThemeId::PaperLight);
        assert_eq!(current_id(), ThemeId::PaperLight);
        assert_eq!(theme().page_bg, PAPER_LIGHT.page_bg);
        set_theme(ThemeId::PaperDark);
        assert_eq!(current_id(), ThemeId::PaperDark);
    }

    #[test]
    fn no_preset_uses_pure_black_or_white() {
        for t in [&PAPER_DARK, &PAPER_LIGHT] {
            let mut all = vec![
                t.page_bg,
                t.ink,
                t.text_muted,
                t.term_fg,
                t.term_bg,
                t.border_normal,
                t.border_focused,
                t.legend_off,
                t.accent_default,
                t.status_fg,
                t.broadcast,
                t.activity,
                t.bell,
                t.dim,
                t.placeholder,
                t.hint_fg,
                t.find_hl_bg,
            ];
            all.extend_from_slice(&t.ansi);
            for c in all {
                assert_ne!(c, (0, 0, 0), "pure black found in a preset");
                assert_ne!(c, (255, 255, 255), "pure white found in a preset");
            }
        }
    }

    #[test]
    fn term_bg_equals_page_bg() {
        for t in [&PAPER_DARK, &PAPER_LIGHT] {
            assert_eq!(t.term_bg, t.page_bg);
        }
    }

    #[test]
    fn term_fg_bg_have_contrast() {
        // crude luminance gap so default text is never near-invisible.
        for t in [&PAPER_DARK, &PAPER_LIGHT] {
            let lum = |c: (u8, u8, u8)| c.0 as i32 + c.1 as i32 + c.2 as i32;
            assert!((lum(t.term_fg) - lum(t.term_bg)).abs() > 200);
        }
    }
}
