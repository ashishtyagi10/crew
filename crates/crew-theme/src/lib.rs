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
    ink: (43, 40, 37),
    text_muted: (90, 84, 75),
    term_fg: (43, 40, 37),
    term_bg: (244, 241, 234),
    border_normal: (201, 194, 178),
    border_focused: (140, 132, 117),
    legend_off: (140, 133, 118),
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
        (163, 133, 63),  // 11 bright yellow
        (88, 116, 148),  // 12 bright blue
        (150, 100, 135), // 13 bright magenta
        (88, 140, 134),  // 14 bright cyan
        (60, 56, 50),    // 15 bright white (boldest ink)
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
}

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "paper-dark",
            ThemeId::PaperLight => "paper-light",
        }
    }

    pub fn from_name(s: &str) -> Option<ThemeId> {
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
            assert_eq!(ThemeId::from_name(id.as_str()), Some(id));
        }
        assert_eq!(ThemeId::from_name("nope"), None);
        assert_eq!(
            ThemeId::from_name("  paper-light "),
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

    #[test]
    fn contrast_thresholds() {
        let cr = contrast_ratio;
        for (name, t) in [("PAPER_DARK", &PAPER_DARK), ("PAPER_LIGHT", &PAPER_LIGHT)] {
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
