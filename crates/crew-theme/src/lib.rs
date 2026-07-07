//! Crew's color themes. A single `Theme` struct holds every UI colour; two
//! `&'static` presets (`PAPER_DARK`, `PAPER_LIGHT`) give crew an e-ink-reader
//! look. The active theme lives behind a lock-free `AtomicU8` so the winit
//! render thread can read it every frame without blocking. No dependencies and
//! no knowledge of the other crates — they import this one.
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

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
    /// Whether this is a dark theme (dark page, light ink). Drives the
    /// random-rotation pool, the light-theme text weight, and grain.
    pub dark: bool,
    /// Grain amplitude multiplier for the paper-texture pass, relative to
    /// the user's configured `paper_grain`. 1.0 on dark themes; 3.0 on
    /// light themes for a visible newsprint texture.
    pub grain: f32,
}

mod presets_crt;
mod presets_paper;
pub use presets_crt::{CRT_AMBER, CRT_BLUE, CRT_GREEN};
pub use presets_paper::{PAPER_DARK, PAPER_LIGHT};

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

    /// Whether this theme is dark — see [`Theme::dark`].
    pub fn is_dark(self) -> bool {
        self.theme().dark
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

/// Random-rotation mode: when on, the active theme changes every `ROTATE_MS`.
static RANDOM: AtomicBool = AtomicBool::new(false);
/// Wall-clock ms of the last rotation (or of enabling random mode).
static ROTATED_MS: AtomicU64 = AtomicU64::new(0);
/// How long each random theme is shown before rotating: 10 minutes.
pub const ROTATE_MS: u64 = 600_000;

/// Whether random-rotation mode is active.
pub fn is_random() -> bool {
    RANDOM.load(Ordering::Relaxed)
}

/// Pick a DARK theme from `ALL_THEMES` that is NOT `current`, deterministically from
/// `seed` (so a caller can seed with a timestamp). Always changes visibly. The pool
/// excludes light themes and `current`.
pub fn random_pick(current: ThemeId, seed: u64) -> ThemeId {
    let others: Vec<ThemeId> = ALL_THEMES
        .iter()
        .copied()
        .filter(|&t| t.is_dark() && t != current)
        .collect();
    // Cheap hash of the seed → index; others is never empty (4 dark themes now, 8 after Task 3).
    let idx = (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % others.len();
    others[idx]
}

/// Enable/disable random-rotation mode. Enabling switches to a random theme
/// immediately (so the effect is visible) and starts the 10-minute clock.
pub fn set_random(on: bool, now_ms: u64) {
    RANDOM.store(on, Ordering::Relaxed);
    if on {
        set_theme(random_pick(current_id(), now_ms));
        ROTATED_MS.store(now_ms, Ordering::Relaxed);
    }
}

/// Called each poll tick with the current wall-clock ms. When random mode is on
/// and `ROTATE_MS` has elapsed since the last rotation, switch to a new random
/// theme and return `true` (so the caller can request a redraw). Cheap and
/// lock-free — safe to call at ~62 Hz on the winit thread.
pub fn tick_random(now_ms: u64) -> bool {
    if !RANDOM.load(Ordering::Relaxed) {
        return false;
    }
    let last = ROTATED_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) < ROTATE_MS {
        return false;
    }
    set_theme(random_pick(current_id(), now_ms));
    ROTATED_MS.store(now_ms, Ordering::Relaxed);
    true
}

/// Advance the Ctrl+Shift+L cycle one step: the 5 themes in ALL_THEMES order,
/// then `random`, wrapping back to the first. Applies the change and returns a
/// label for the status line (`"random"` or a theme's `as_str()`).
pub fn cycle_next(now_ms: u64) -> &'static str {
    if is_random() {
        set_random(false, now_ms);
        set_theme(ALL_THEMES[0]);
        ALL_THEMES[0].as_str()
    } else {
        let cur = current_id();
        let i = ALL_THEMES.iter().position(|&t| t == cur).unwrap_or(0);
        if i + 1 < ALL_THEMES.len() {
            set_theme(ALL_THEMES[i + 1]);
            ALL_THEMES[i + 1].as_str()
        } else {
            set_random(true, now_ms); // last fixed theme → enter random mode
            "random"
        }
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
