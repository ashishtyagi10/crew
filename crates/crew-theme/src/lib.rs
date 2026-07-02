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
    /// Rounded pane border stroke width, in physical pixels.
    pub border_thickness: f32,
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

/// High-contrast monochrome ("newspaper") dark theme — near-black/near-white
/// chrome for maximum legibility with minimal glare. Terminal ANSI output
/// keeps muted-but-readable colours so error/diff colour cues survive.
/// The default.
pub static PAPER_DARK: Theme = Theme {
    page_bg: (8, 8, 8),
    ink: (247, 247, 247),
    text_muted: (196, 196, 196),
    term_fg: (247, 247, 247),
    term_bg: (8, 8, 8),
    // Unfocused borders sit back (~3.4:1 on the page — visual parity with the
    // light theme's ~1.9:1 weight) so the FOCUSED near-white frame carries the
    // "where am I" signal instead of every card shouting equally.
    border_normal: (72, 72, 72),
    border_focused: (235, 235, 235),
    border_thickness: 2.5,
    legend_off: (140, 140, 140),
    accent_default: (240, 240, 240),
    status_fg: (235, 195, 120),
    broadcast: (200, 150, 190),
    activity: (140, 175, 210),
    bell: (235, 195, 120),
    dim: (125, 125, 125),
    placeholder: (112, 112, 112),
    hint_fg: (135, 135, 135),
    find_hl_bg: (70, 62, 20),
    ansi: [
        (95, 95, 95),    // 0  black -> neutral grey (visible on near-black)
        (235, 105, 90),  // 1  red
        (140, 220, 110), // 2  green
        (235, 200, 90),  // 3  yellow
        (120, 180, 235), // 4  blue
        (215, 140, 215), // 5  magenta
        (110, 220, 215), // 6  cyan
        (225, 225, 225), // 7  white -> neutral light grey
        (140, 140, 140), // 8  bright black
        (255, 130, 110), // 9  bright red
        (170, 240, 130), // 10 bright green
        (255, 220, 110), // 11 bright yellow
        (145, 200, 255), // 12 bright blue
        (235, 165, 235), // 13 bright magenta
        (135, 245, 235), // 14 bright cyan
        (250, 250, 250), // 15 bright white
    ],
};

/// Warm paper "day" page — soft off-white with ink-toned output.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (246, 243, 236),
    // Ink and every text shade run deep enough that type reads crisp on the
    // bright page (ink ≥ 16:1, muted ≥ 11:1) rather than washed-out.
    ink: (22, 20, 18),
    text_muted: (55, 51, 45),
    term_fg: (22, 20, 18),
    term_bg: (246, 243, 236),
    border_normal: (175, 166, 148),
    border_focused: (105, 97, 83),
    border_thickness: 3.0,
    legend_off: (100, 94, 83),
    accent_default: (110, 72, 38),
    status_fg: (107, 78, 26),
    broadcast: (110, 45, 88),
    activity: (40, 72, 108),
    bell: (115, 84, 20),
    dim: (105, 99, 88),
    placeholder: (118, 112, 101),
    hint_fg: (112, 106, 95),
    find_hl_bg: (235, 220, 150),
    ansi: [
        (28, 26, 23),   // 0  black
        (152, 36, 28),  // 1  red (brick)
        (58, 92, 30),   // 2  green (sage)
        (140, 96, 20),  // 3  yellow (ochre)
        (36, 74, 116),  // 4  blue (faded indigo)
        (110, 44, 96),  // 5  magenta (mauve)
        (18, 96, 92),   // 6  cyan (teal)
        (70, 66, 58),   // 7  white (warm gray)
        (92, 87, 76),   // 8  bright black
        (176, 48, 36),  // 9  bright red
        (74, 110, 40),  // 10 bright green
        (158, 108, 24), // 11 bright yellow
        (48, 92, 140),  // 12 bright blue
        (128, 58, 112), // 13 bright magenta
        (24, 114, 108), // 14 bright cyan
        (30, 28, 25),   // 15 bright white (boldest ink)
    ],
};

/// **Neon green phosphor** (P1, electrified): hot Tron-grid green on a
/// near-black tube, with a monochrome-green ANSI palette (brightness tiers,
/// faint hue tilts) for that single-gun terminal look. The paper-grain pass
/// reads as a subtle glow.
pub static CRT_GREEN: Theme = Theme {
    page_bg: (3, 10, 5),
    ink: (0, 255, 102),
    text_muted: (0, 204, 82),
    term_fg: (0, 255, 102),
    term_bg: (3, 10, 5),
    border_normal: (0, 115, 55),
    border_focused: (0, 255, 140),
    border_thickness: 2.5,
    legend_off: (0, 160, 70),
    accent_default: (64, 255, 160),
    status_fg: (190, 255, 80),
    broadcast: (150, 255, 150),
    activity: (0, 230, 120),
    bell: (200, 255, 90),
    dim: (0, 110, 55),
    placeholder: (0, 135, 60),
    hint_fg: (0, 150, 66),
    find_hl_bg: (10, 70, 30),
    ansi: [
        (10, 45, 20),    // 0  black
        (170, 255, 70),  // 1  red
        (0, 255, 102),   // 2  green
        (200, 255, 80),  // 3  yellow
        (0, 230, 170),   // 4  blue
        (130, 255, 150), // 5  magenta
        (0, 255, 200),   // 6  cyan
        (170, 255, 190), // 7  white
        (0, 140, 70),    // 8  bright black
        (200, 255, 100), // 9  bright red
        (80, 255, 130),  // 10 bright green
        (230, 255, 110), // 11 bright yellow
        (60, 255, 200),  // 12 bright blue
        (170, 255, 180), // 13 bright magenta
        (100, 255, 230), // 14 bright cyan
        (210, 255, 220), // 15 bright white
    ],
};

/// **Neon amber phosphor** (P3, electrified): saturated Tron-orange amber on a
/// near-black tube — the warm counterpart of the green grid.
pub static CRT_AMBER: Theme = Theme {
    page_bg: (14, 8, 2),
    ink: (255, 184, 0),
    text_muted: (226, 148, 0),
    term_fg: (255, 184, 0),
    term_bg: (14, 8, 2),
    border_normal: (145, 90, 20),
    border_focused: (255, 170, 40),
    border_thickness: 2.5,
    legend_off: (180, 115, 20),
    accent_default: (255, 210, 60),
    status_fg: (255, 200, 70),
    broadcast: (255, 170, 110),
    activity: (255, 170, 50),
    bell: (255, 190, 40),
    dim: (130, 85, 25),
    placeholder: (155, 100, 25),
    hint_fg: (172, 110, 25),
    find_hl_bg: (75, 48, 10),
    ansi: [
        (60, 35, 10),    // 0  black
        (255, 120, 40),  // 1  red
        (240, 200, 40),  // 2  green
        (255, 200, 30),  // 3  yellow
        (255, 160, 90),  // 4  blue
        (255, 140, 90),  // 5  magenta
        (250, 190, 110), // 6  cyan
        (255, 205, 120), // 7  white
        (150, 95, 35),   // 8  bright black
        (255, 140, 60),  // 9  bright red
        (255, 220, 60),  // 10 bright green
        (255, 215, 70),  // 11 bright yellow
        (255, 180, 110), // 12 bright blue
        (255, 160, 110), // 13 bright magenta
        (255, 210, 140), // 14 bright cyan
        (255, 225, 160), // 15 bright white
    ],
};

/// **Neon blue phosphor** (electrified): Tron light-cycle cyan on a
/// near-black tube — electric edge-glow blues, the coolest of the three grids.
pub static CRT_BLUE: Theme = Theme {
    page_bg: (2, 8, 18),
    ink: (0, 229, 255),
    text_muted: (0, 182, 214),
    term_fg: (0, 229, 255),
    term_bg: (2, 8, 18),
    border_normal: (0, 100, 140),
    border_focused: (0, 215, 255),
    border_thickness: 2.5,
    legend_off: (0, 145, 180),
    accent_default: (120, 255, 255),
    status_fg: (150, 230, 255),
    broadcast: (170, 180, 255),
    activity: (0, 200, 240),
    bell: (170, 220, 255),
    dim: (0, 105, 140),
    placeholder: (0, 122, 155),
    hint_fg: (0, 138, 172),
    find_hl_bg: (10, 45, 75),
    ansi: [
        (20, 50, 75),    // 0  black
        (150, 170, 255), // 1  red
        (0, 255, 220),   // 2  green
        (140, 220, 255), // 3  yellow
        (60, 160, 255),  // 4  blue
        (150, 150, 255), // 5  magenta
        (0, 240, 255),   // 6  cyan
        (170, 225, 255), // 7  white
        (0, 120, 170),   // 8  bright black
        (180, 190, 255), // 9  bright red
        (60, 255, 235),  // 10 bright green
        (170, 235, 255), // 11 bright yellow
        (90, 190, 255),  // 12 bright blue
        (180, 170, 255), // 13 bright magenta
        (110, 250, 255), // 14 bright cyan
        (200, 240, 255), // 15 bright white
    ],
};

/// WCAG 2.1 contrast ratio between two sRGB colours.
pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
    let lin = |c: u8| -> f32 {
        let x = c as f32 / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    let lum =
        |c: (u8, u8, u8)| -> f32 { 0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2) };
    let l1 = lum(a);
    let l2 = lum(b);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    PaperDark,
    PaperLight,
    CrtGreen,
    CrtAmber,
    CrtBlue,
}

/// Every theme, in cycle order (used by the `Ctrl+Shift+L` rotation and the
/// `/theme` completion). Keep in sync with the enum.
pub const ALL_THEMES: [ThemeId; 5] = [
    ThemeId::PaperDark,
    ThemeId::PaperLight,
    ThemeId::CrtGreen,
    ThemeId::CrtAmber,
    ThemeId::CrtBlue,
];

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "paper-dark",
            ThemeId::PaperLight => "paper-light",
            ThemeId::CrtGreen => "crt-green",
            ThemeId::CrtAmber => "crt-amber",
            ThemeId::CrtBlue => "crt-blue",
        }
    }

    /// A short human description, for the `/theme` value picker.
    pub fn describe(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "high-contrast newspaper (dark)",
            ThemeId::PaperLight => "warm paper page (light)",
            ThemeId::CrtGreen => "neon green phosphor CRT",
            ThemeId::CrtAmber => "neon amber phosphor CRT",
            ThemeId::CrtBlue => "neon blue phosphor CRT (Tron)",
        }
    }

    pub fn from_name(s: &str) -> Option<ThemeId> {
        match s.trim() {
            "paper-dark" => Some(ThemeId::PaperDark),
            "paper-light" => Some(ThemeId::PaperLight),
            "crt-green" => Some(ThemeId::CrtGreen),
            "crt-amber" => Some(ThemeId::CrtAmber),
            "crt-blue" => Some(ThemeId::CrtBlue),
            _ => None,
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::PaperDark => &PAPER_DARK,
            ThemeId::PaperLight => &PAPER_LIGHT,
            ThemeId::CrtGreen => &CRT_GREEN,
            ThemeId::CrtAmber => &CRT_AMBER,
            ThemeId::CrtBlue => &CRT_BLUE,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            ThemeId::PaperDark => 0,
            ThemeId::PaperLight => 1,
            ThemeId::CrtGreen => 2,
            ThemeId::CrtAmber => 3,
            ThemeId::CrtBlue => 4,
        }
    }

    fn from_u8(v: u8) -> ThemeId {
        match v {
            1 => ThemeId::PaperLight,
            2 => ThemeId::CrtGreen,
            3 => ThemeId::CrtAmber,
            4 => ThemeId::CrtBlue,
            _ => ThemeId::PaperDark,
        }
    }

    /// The next theme in [`ALL_THEMES`] order, wrapping — the `Ctrl+Shift+L` step.
    pub fn next(self) -> ThemeId {
        let i = ALL_THEMES.iter().position(|&t| t == self).unwrap_or(0);
        ALL_THEMES[(i + 1) % ALL_THEMES.len()]
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
        for id in ALL_THEMES {
            assert_eq!(ThemeId::from_name(id.as_str()), Some(id));
        }
        assert_eq!(ThemeId::from_name("nope"), None);
        assert_eq!(
            ThemeId::from_name("  paper-light "),
            Some(ThemeId::PaperLight)
        );
        assert_eq!(ThemeId::from_name("crt-green"), Some(ThemeId::CrtGreen));
    }

    #[test]
    fn next_cycles_through_all_and_wraps() {
        // Every theme steps to another, and stepping the whole ring returns home.
        let mut id = ThemeId::PaperDark;
        for _ in 0..ALL_THEMES.len() {
            id = id.next();
        }
        assert_eq!(id, ThemeId::PaperDark);
        assert_eq!(ThemeId::CrtBlue.next(), ThemeId::PaperDark); // last wraps to first
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
        for id in ALL_THEMES {
            let t = id.theme();
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
        for id in ALL_THEMES {
            let t = id.theme();
            assert_eq!(t.term_bg, t.page_bg);
        }
    }

    #[test]
    fn term_fg_bg_have_contrast() {
        // crude luminance gap so default text is never near-invisible.
        for id in ALL_THEMES {
            let t = id.theme();
            let lum = |c: (u8, u8, u8)| c.0 as i32 + c.1 as i32 + c.2 as i32;
            assert!((lum(t.term_fg) - lum(t.term_bg)).abs() > 200);
        }
    }

    #[test]
    fn contrast_thresholds() {
        let cr = contrast_ratio;
        for id in ALL_THEMES {
            let name = id.as_str();
            let t = id.theme();
            let bg = t.page_bg;
            let tbg = t.term_bg;

            assert!(
                cr(t.ink, bg) >= 10.0,
                "{name}: ink vs page_bg = {:.3} (need >= 10.0)",
                cr(t.ink, bg)
            );
            assert!(
                cr(t.term_fg, tbg) >= 10.0,
                "{name}: term_fg vs term_bg = {:.3} (need >= 10.0)",
                cr(t.term_fg, tbg)
            );
            assert!(
                cr(t.text_muted, bg) >= 7.0,
                "{name}: text_muted vs page_bg = {:.3} (need >= 7.0)",
                cr(t.text_muted, bg)
            );
            assert!(
                cr(t.legend_off, bg) >= 3.0,
                "{name}: legend_off vs page_bg = {:.3} (need >= 3.0)",
                cr(t.legend_off, bg)
            );
            assert!(
                cr(t.hint_fg, bg) >= 2.5,
                "{name}: hint_fg vs page_bg = {:.3} (need >= 2.5)",
                cr(t.hint_fg, bg)
            );
            assert!(
                cr(t.placeholder, bg) >= 2.3,
                "{name}: placeholder vs page_bg = {:.3} (need >= 2.3)",
                cr(t.placeholder, bg)
            );
            assert!(
                cr(t.accent_default, bg) >= 3.0,
                "{name}: accent_default vs page_bg = {:.3} (need >= 3.0)",
                cr(t.accent_default, bg)
            );
            assert!(
                cr(t.border_focused, bg) >= 2.2,
                "{name}: border_focused vs page_bg = {:.3} (need >= 2.2)",
                cr(t.border_focused, bg)
            );
            assert!(
                cr(t.border_normal, bg) >= 1.45,
                "{name}: border_normal vs page_bg = {:.3} (need >= 1.45)",
                cr(t.border_normal, bg)
            );
            // ANSI terminal colours (skip slots 0, 7, 8, 15 = blacks and whites)
            for i in [1usize, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14] {
                assert!(
                    cr(t.ansi[i], tbg) >= 3.0,
                    "{name}: ansi[{i}] {:?} vs term_bg = {:.3} (need >= 3.0)",
                    t.ansi[i],
                    cr(t.ansi[i], tbg)
                );
            }
        }
    }
}
