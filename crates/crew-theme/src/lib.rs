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
    page_bg: (10, 10, 10),
    ink: (236, 236, 236),
    text_muted: (175, 175, 175),
    term_fg: (236, 236, 236),
    term_bg: (10, 10, 10),
    border_normal: (85, 85, 85),
    border_focused: (205, 205, 205),
    border_thickness: 2.5,
    legend_off: (120, 120, 120),
    accent_default: (230, 230, 230),
    status_fg: (210, 180, 120),
    broadcast: (181, 138, 168),
    activity: (125, 154, 184),
    bell: (210, 180, 120),
    dim: (110, 110, 110),
    placeholder: (95, 95, 95),
    hint_fg: (120, 120, 120),
    find_hl_bg: (60, 55, 20),
    ansi: [
        (90, 90, 90),    // 0  black -> neutral grey (visible on near-black)
        (210, 120, 105), // 1  red
        (160, 185, 110), // 2  green
        (215, 180, 110), // 3  yellow
        (130, 165, 200), // 4  blue
        (190, 145, 180), // 5  magenta
        (135, 190, 185), // 6  cyan
        (220, 220, 220), // 7  white -> neutral light grey
        (130, 130, 130), // 8  bright black
        (225, 140, 120), // 9  bright red
        (180, 200, 130), // 10 bright green
        (230, 200, 135), // 11 bright yellow
        (150, 185, 215), // 12 bright blue
        (210, 165, 200), // 13 bright magenta
        (155, 205, 200), // 14 bright cyan
        (240, 240, 240), // 15 bright white
    ],
};

/// Warm paper "day" page — soft off-white with ink-toned output.
pub static PAPER_LIGHT: Theme = Theme {
    page_bg: (244, 241, 234),
    // Ink and every text shade run ~18% darker than a flat paper palette so
    // type reads crisp on the bright page rather than washed-out.
    ink: (35, 33, 30),
    text_muted: (74, 69, 62),
    term_fg: (35, 33, 30),
    term_bg: (244, 241, 234),
    border_normal: (201, 194, 178),
    border_focused: (140, 132, 117),
    border_thickness: 3.0,
    legend_off: (115, 109, 97),
    accent_default: (128, 88, 52),
    status_fg: (123, 90, 33),
    broadcast: (123, 57, 98),
    activity: (49, 82, 115),
    bell: (131, 98, 33),
    dim: (115, 108, 97),
    placeholder: (131, 125, 113),
    hint_fg: (131, 125, 113),
    find_hl_bg: (232, 220, 168),
    ansi: [
        (35, 33, 30),   // 0  black
        (128, 48, 38),  // 1  red (brick)
        (76, 88, 48),   // 2  green (sage)
        (126, 101, 38), // 3  yellow (ochre)
        (52, 74, 98),   // 4  blue (faded indigo)
        (102, 62, 90),  // 5  magenta (mauve)
        (52, 91, 88),   // 6  cyan (teal)
        (75, 71, 62),   // 7  white (warm gray)
        (98, 93, 81),   // 8  bright black
        (146, 67, 54),  // 9  bright red
        (100, 110, 67), // 10 bright green
        (134, 109, 52), // 11 bright yellow
        (72, 95, 121),  // 12 bright blue
        (123, 82, 111), // 13 bright magenta
        (72, 115, 110), // 14 bright cyan
        (49, 46, 41),   // 15 bright white (boldest ink)
    ],
};

/// Old-school **CRT green phosphor** (P1): bright green on near-black, with a
/// monochrome-green ANSI palette (brightness tiers, faint hue tilts) for that
/// single-gun terminal look. The paper-grain pass reads as a subtle glow.
pub static CRT_GREEN: Theme = Theme {
    page_bg: (6, 16, 8),
    ink: (80, 255, 100),
    text_muted: (70, 200, 90),
    term_fg: (80, 255, 100),
    term_bg: (6, 16, 8),
    border_normal: (40, 110, 55),
    border_focused: (90, 220, 120),
    border_thickness: 2.5,
    legend_off: (60, 150, 75),
    accent_default: (120, 255, 150),
    status_fg: (180, 230, 90),
    broadcast: (140, 200, 120),
    activity: (90, 210, 130),
    bell: (180, 230, 100),
    dim: (55, 120, 65),
    placeholder: (60, 130, 70),
    hint_fg: (70, 150, 85),
    find_hl_bg: (20, 60, 25),
    ansi: [
        (25, 50, 30),    // 0  black
        (110, 200, 90),  // 1  red
        (70, 215, 90),   // 2  green
        (150, 220, 90),  // 3  yellow
        (70, 200, 150),  // 4  blue
        (130, 205, 130), // 5  magenta
        (80, 220, 175),  // 6  cyan
        (150, 235, 165), // 7  white
        (60, 120, 75),   // 8  bright black
        (140, 230, 110), // 9  bright red
        (100, 255, 120), // 10 bright green
        (180, 245, 110), // 11 bright yellow
        (100, 230, 180), // 12 bright blue
        (160, 235, 160), // 13 bright magenta
        (110, 250, 205), // 14 bright cyan
        (180, 255, 190), // 15 bright white
    ],
};

/// **CRT amber phosphor** (P3): the warm-amber variation of the green tube.
pub static CRT_AMBER: Theme = Theme {
    page_bg: (18, 12, 4),
    ink: (255, 176, 70),
    text_muted: (215, 145, 55),
    term_fg: (255, 176, 70),
    term_bg: (18, 12, 4),
    border_normal: (120, 78, 30),
    border_focused: (225, 150, 65),
    border_thickness: 2.5,
    legend_off: (160, 105, 40),
    accent_default: (255, 185, 80),
    status_fg: (230, 175, 70),
    broadcast: (210, 150, 90),
    activity: (225, 160, 70),
    bell: (240, 170, 60),
    dim: (120, 82, 35),
    placeholder: (135, 92, 40),
    hint_fg: (155, 105, 45),
    find_hl_bg: (60, 40, 12),
    ansi: [
        (55, 38, 15),    // 0  black
        (230, 130, 70),  // 1  red
        (200, 175, 60),  // 2  green
        (235, 190, 70),  // 3  yellow
        (200, 150, 110), // 4  blue
        (225, 150, 110), // 5  magenta
        (205, 175, 120), // 6  cyan
        (235, 190, 130), // 7  white
        (120, 85, 40),   // 8  bright black
        (255, 150, 90),  // 9  bright red
        (225, 205, 80),  // 10 bright green
        (255, 205, 85),  // 11 bright yellow
        (225, 180, 140), // 12 bright blue
        (245, 175, 135), // 13 bright magenta
        (230, 200, 150), // 14 bright cyan
        (255, 215, 160), // 15 bright white
    ],
};

/// **CRT blue phosphor**: a cool cyan-blue tube, the third phosphor variation.
pub static CRT_BLUE: Theme = Theme {
    page_bg: (5, 12, 20),
    ink: (120, 210, 255),
    text_muted: (95, 170, 215),
    term_fg: (120, 210, 255),
    term_bg: (5, 12, 20),
    border_normal: (40, 85, 120),
    border_focused: (100, 180, 225),
    border_thickness: 2.5,
    legend_off: (70, 130, 170),
    accent_default: (120, 205, 255),
    status_fg: (150, 200, 240),
    broadcast: (150, 170, 220),
    activity: (110, 190, 235),
    bell: (170, 200, 245),
    dim: (55, 100, 135),
    placeholder: (65, 110, 145),
    hint_fg: (75, 130, 170),
    find_hl_bg: (18, 40, 60),
    ansi: [
        (30, 50, 70),    // 0  black
        (150, 160, 220), // 1  red
        (90, 210, 210),  // 2  green
        (160, 205, 200), // 3  yellow
        (90, 160, 240),  // 4  blue
        (150, 165, 235), // 5  magenta
        (90, 215, 240),  // 6  cyan
        (160, 205, 240), // 7  white
        (70, 110, 150),  // 8  bright black
        (170, 180, 245), // 9  bright red
        (110, 235, 235), // 10 bright green
        (180, 225, 220), // 11 bright yellow
        (110, 180, 255), // 12 bright blue
        (175, 185, 250), // 13 bright magenta
        (120, 240, 255), // 14 bright cyan
        (185, 225, 255), // 15 bright white
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
                cr(t.ink, bg) >= 7.0,
                "{name}: ink vs page_bg = {:.3} (need >= 7.0)",
                cr(t.ink, bg)
            );
            assert!(
                cr(t.term_fg, tbg) >= 7.0,
                "{name}: term_fg vs term_bg = {:.3} (need >= 7.0)",
                cr(t.term_fg, tbg)
            );
            assert!(
                cr(t.text_muted, bg) >= 4.5,
                "{name}: text_muted vs page_bg = {:.3} (need >= 4.5)",
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
