//! Crew's color themes. A single `Theme` struct holds every UI colour; two
//! `&'static` presets (`PAPER_DARK`, `PAPER_LIGHT`) give crew an e-ink-reader
//! look. The active theme lives behind a lock-free `AtomicU8` so the winit
//! render thread can read it every frame without blocking. No dependencies and
//! no knowledge of the other crates — they import this one.
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

mod fonts;
pub use fonts::{font_prefs, FONT_ALLOWLIST};

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
    /// the user's configured `paper_grain`. 1.0 on dark themes; 1.2 on
    /// light themes for a visible newsprint texture (calibrated for the
    /// gamma-space blend — see presets_paper.rs).
    pub grain: f32,
    /// Whether this theme reads as a CRT phosphor tube. When true — and unless
    /// the user overrides it with `/crt off` — the renderer wraps the frame in
    /// the CRT post-process (curvature, scanlines, phosphor glow, corner
    /// darkening). Only the `CRT_*` presets set this; every paper theme is
    /// `false` so the crisp flat look is the default.
    pub crt: bool,
}

mod presets_crt;
mod presets_paper;
mod presets_paper_light;
pub use presets_crt::{CRT_AMBER, CRT_BLUE, CRT_GREEN, CRT_VIOLET};
pub use presets_paper::{GRAPHITE, MIDNIGHT_INK, PAPER_DARK, PAPER_LIGHT, SEPIA_DARK};
pub use presets_paper_light::{COLDPRESS_GRAY, IVORY_LEDGER, SALMON_BROADSHEET, SEPIA_LIGHT};

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
    SepiaDark,
    SepiaLight,
    MidnightInk,
    Graphite,
    ColdpressGray,
    SalmonBroadsheet,
    IvoryLedger,
    CrtGreen,
    CrtAmber,
    CrtBlue,
    CrtViolet,
}

/// Every theme, in cycle order (used by the `Ctrl+Shift+L` rotation and the
/// `/theme` completion). Keep in sync with the enum.
pub const ALL_THEMES: [ThemeId; 13] = [
    ThemeId::PaperDark,
    ThemeId::PaperLight,
    ThemeId::SepiaDark,
    ThemeId::SepiaLight,
    ThemeId::MidnightInk,
    ThemeId::Graphite,
    ThemeId::ColdpressGray,
    ThemeId::SalmonBroadsheet,
    ThemeId::IvoryLedger,
    ThemeId::CrtGreen,
    ThemeId::CrtAmber,
    ThemeId::CrtBlue,
    ThemeId::CrtViolet,
];

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "paper-dark",
            ThemeId::PaperLight => "paper-light",
            ThemeId::SepiaDark => "sepia-dark",
            ThemeId::SepiaLight => "sepia-light",
            ThemeId::MidnightInk => "midnight-ink",
            ThemeId::Graphite => "graphite",
            ThemeId::ColdpressGray => "coldpress-gray",
            ThemeId::SalmonBroadsheet => "salmon-broadsheet",
            ThemeId::IvoryLedger => "ivory-ledger",
            ThemeId::CrtGreen => "crt-green",
            ThemeId::CrtAmber => "crt-amber",
            ThemeId::CrtBlue => "crt-blue",
            ThemeId::CrtViolet => "crt-violet",
        }
    }

    /// A short human description, for the `/theme` value picker.
    pub fn describe(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "high-contrast newspaper (dark)",
            ThemeId::PaperLight => "warm paper page (light)",
            ThemeId::SepiaDark => "dark sepia paper (warm cream ink)",
            ThemeId::SepiaLight => "aged-newsprint cream page (light sepia)",
            ThemeId::MidnightInk => "deep navy page, cool off-white ink",
            ThemeId::Graphite => "soft charcoal paper (gentle dark)",
            ThemeId::ColdpressGray => "cool pale-gray page (light graphite)",
            ThemeId::SalmonBroadsheet => "FT salmon-pink broadsheet (light)",
            ThemeId::IvoryLedger => "ivory page, ledger-green ink (light)",
            ThemeId::CrtGreen => "neon green phosphor CRT",
            ThemeId::CrtAmber => "neon amber phosphor CRT",
            ThemeId::CrtBlue => "neon blue phosphor CRT (Tron)",
            ThemeId::CrtViolet => "neon violet phosphor CRT",
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
            "sepia-dark" => Some(ThemeId::SepiaDark),
            "sepia-light" => Some(ThemeId::SepiaLight),
            "midnight-ink" => Some(ThemeId::MidnightInk),
            "graphite" => Some(ThemeId::Graphite),
            "coldpress-gray" => Some(ThemeId::ColdpressGray),
            "salmon-broadsheet" => Some(ThemeId::SalmonBroadsheet),
            "ivory-ledger" => Some(ThemeId::IvoryLedger),
            "crt-green" => Some(ThemeId::CrtGreen),
            "crt-amber" => Some(ThemeId::CrtAmber),
            "crt-blue" => Some(ThemeId::CrtBlue),
            "crt-violet" => Some(ThemeId::CrtViolet),
            _ => None,
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::PaperDark => &PAPER_DARK,
            ThemeId::PaperLight => &PAPER_LIGHT,
            ThemeId::SepiaDark => &SEPIA_DARK,
            ThemeId::SepiaLight => &SEPIA_LIGHT,
            ThemeId::MidnightInk => &MIDNIGHT_INK,
            ThemeId::Graphite => &GRAPHITE,
            ThemeId::ColdpressGray => &COLDPRESS_GRAY,
            ThemeId::SalmonBroadsheet => &SALMON_BROADSHEET,
            ThemeId::IvoryLedger => &IVORY_LEDGER,
            ThemeId::CrtGreen => &CRT_GREEN,
            ThemeId::CrtAmber => &CRT_AMBER,
            ThemeId::CrtBlue => &CRT_BLUE,
            ThemeId::CrtViolet => &CRT_VIOLET,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            ThemeId::PaperDark => 0,
            ThemeId::PaperLight => 1,
            ThemeId::CrtGreen => 2,
            ThemeId::CrtAmber => 3,
            ThemeId::CrtBlue => 4,
            ThemeId::SepiaDark => 5,
            ThemeId::MidnightInk => 6,
            ThemeId::Graphite => 7,
            ThemeId::CrtViolet => 8,
            ThemeId::SepiaLight => 9,
            ThemeId::SalmonBroadsheet => 10,
            ThemeId::ColdpressGray => 11,
            ThemeId::IvoryLedger => 12,
        }
    }

    fn from_u8(v: u8) -> ThemeId {
        match v {
            1 => ThemeId::PaperLight,
            2 => ThemeId::CrtGreen,
            3 => ThemeId::CrtAmber,
            4 => ThemeId::CrtBlue,
            5 => ThemeId::SepiaDark,
            6 => ThemeId::MidnightInk,
            7 => ThemeId::Graphite,
            8 => ThemeId::CrtViolet,
            9 => ThemeId::SepiaLight,
            10 => ThemeId::SalmonBroadsheet,
            11 => ThemeId::ColdpressGray,
            12 => ThemeId::IvoryLedger,
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

/// Rotation mode: when set, the active theme changes every [`ROTATE_MS`]
/// within the mode's pool. Stored as a lock-free u8 for per-frame reads:
/// 0 = off, 1 = dark pool, 2 = light pool, 3 = auto (pool follows the OS
/// appearance via [`set_os_dark`]).
static MODE: AtomicU8 = AtomicU8::new(0);
/// Wall-clock ms of the last rotation (or of enabling a mode).
static ROTATED_MS: AtomicU64 = AtomicU64::new(0);
/// The OS appearance, fed by winit's ThemeChanged. Defaults to dark so a
/// platform that never reports stays on the dark pool.
static OS_DARK: AtomicBool = AtomicBool::new(true);
/// How long each rotated theme is shown: 10 minutes (fonts share this).
pub const ROTATE_MS: u64 = 600_000;

/// A rotating theme: each mode owns a pool of palettes and cycles through them
/// every [`ROTATE_MS`]. These ARE crew's themes now — the individual palettes
/// (`PAPER_DARK`, `CRT_GREEN`, …) are the pool members, no longer offered on
/// their own. `Auto` is an unlisted back-compat mode (its pool follows the OS
/// appearance); [`THEME_MODES`] is the three the picker advertises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomMode {
    Dark,
    Light,
    Crt,
    Auto,
}

/// The three themes crew offers — each a rotation over its own pool. This is
/// the whole user-facing theme list (`/theme`, the settings picker, the
/// `Ctrl+Shift+L` cycle); everything else parses for back-compat but isn't
/// advertised.
pub const THEME_MODES: [RandomMode; 3] = [RandomMode::Dark, RandomMode::Light, RandomMode::Crt];

impl RandomMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RandomMode::Dark => "dark",
            RandomMode::Light => "light",
            RandomMode::Crt => "crt",
            RandomMode::Auto => "auto",
        }
    }

    /// A short human description, for the `/theme` value picker and listings.
    pub fn describe(self) -> &'static str {
        match self {
            RandomMode::Dark => "rotating dark paper themes",
            RandomMode::Light => "rotating light paper themes",
            RandomMode::Crt => "rotating CRT phosphor themes",
            RandomMode::Auto => "light by day, dark by night \u{2014} follows the OS",
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            RandomMode::Dark => 1,
            RandomMode::Light => 2,
            RandomMode::Auto => 3,
            RandomMode::Crt => 4,
        }
    }

    fn from_u8(v: u8) -> Option<RandomMode> {
        match v {
            1 => Some(RandomMode::Dark),
            2 => Some(RandomMode::Light),
            3 => Some(RandomMode::Auto),
            4 => Some(RandomMode::Crt),
            _ => None,
        }
    }

    /// Whether `id` belongs to this mode's rotation pool. Every palette lands
    /// in exactly one of Dark/Light/Crt (CRT palettes are `dark` too, so the
    /// `!crt` guard keeps them out of the plain dark pool); `Auto` borrows the
    /// dark or light pool depending on the OS appearance.
    fn in_pool(self, id: ThemeId) -> bool {
        let t = id.theme();
        match self {
            RandomMode::Dark => t.dark && !t.crt,
            RandomMode::Light => !t.dark && !t.crt,
            RandomMode::Crt => t.crt,
            RandomMode::Auto => !t.crt && t.dark == os_dark(),
        }
    }
}

/// What a theme name string resolves to: a fixed theme or a rotation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Fixed(ThemeId),
    Mode(RandomMode),
}

/// Parse a `/theme` argument / config value. The three canonical names are
/// `dark`, `light`, `crt`; the pre-consolidation names (`random`,
/// `random-dark`, `random-light`, `auto`) and every individual palette name
/// still parse so old configs keep loading.
pub fn parse_selection(s: &str) -> Option<Selection> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("dark")
        || s.eq_ignore_ascii_case("random")
        || s.eq_ignore_ascii_case("random-dark")
    {
        return Some(Selection::Mode(RandomMode::Dark));
    }
    if s.eq_ignore_ascii_case("light") || s.eq_ignore_ascii_case("random-light") {
        return Some(Selection::Mode(RandomMode::Light));
    }
    if s.eq_ignore_ascii_case("crt") || s.eq_ignore_ascii_case("random-crt") {
        return Some(Selection::Mode(RandomMode::Crt));
    }
    if s.eq_ignore_ascii_case("auto") {
        return Some(Selection::Mode(RandomMode::Auto));
    }
    ThemeId::from_name(s).map(Selection::Fixed)
}

/// The active rotation mode, if any.
pub fn mode() -> Option<RandomMode> {
    RandomMode::from_u8(MODE.load(Ordering::Relaxed))
}

/// Whether any rotation mode is active.
pub fn is_random() -> bool {
    mode().is_some()
}

/// Report the OS appearance (winit ThemeChanged / startup Window::theme).
/// While `auto` is active the change takes effect on the next rotation tick;
/// callers that want an immediate flip re-apply the selection (the app does).
pub fn set_os_dark(dark: bool) {
    OS_DARK.store(dark, Ordering::Relaxed);
}

/// The last reported OS appearance (defaults to dark).
pub fn os_dark() -> bool {
    OS_DARK.load(Ordering::Relaxed)
}

/// Pick a theme from `mode`'s pool that is NOT `current`, deterministically
/// from `seed`. Every pool has ≥ 4 entries, so minus `current` it is never
/// empty; the `current` filter is skipped only in the impossible case where it
/// would empty the pool (keeps the modulo safe).
pub fn random_pick(current: ThemeId, seed: u64, mode: RandomMode) -> ThemeId {
    let mut others: Vec<ThemeId> = ALL_THEMES
        .iter()
        .copied()
        .filter(|&t| mode.in_pool(t) && t != current)
        .collect();
    if others.is_empty() {
        others = ALL_THEMES
            .iter()
            .copied()
            .filter(|&t| mode.in_pool(t))
            .collect();
    }
    let idx = (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % others.len();
    others[idx]
}

/// Apply a parsed selection: a fixed theme pins it (mode off); a mode
/// switches immediately to a pick from its pool (so the effect is visible)
/// and starts the 10-minute clock.
pub fn apply_selection(sel: Selection, now_ms: u64) {
    match sel {
        Selection::Fixed(id) => {
            MODE.store(0, Ordering::Relaxed);
            set_theme(id);
        }
        Selection::Mode(m) => {
            MODE.store(m.as_u8(), Ordering::Relaxed);
            set_theme(random_pick(current_id(), now_ms, m));
            ROTATED_MS.store(now_ms, Ordering::Relaxed);
        }
    }
}

/// The status-line label for the active selection: the mode's name while
/// rotating, else the pinned theme's name.
pub fn selection_label() -> &'static str {
    match mode() {
        Some(m) => m.as_str(),
        None => current_id().as_str(),
    }
}

/// Called each poll tick with the current wall-clock ms. When a mode is on
/// and `ROTATE_MS` has elapsed, switch to a new pick from the mode's pool
/// (auto re-reads the OS appearance every tick) and return `true` so the
/// caller repaints. Cheap and lock-free — safe at ~62 Hz on the winit thread.
pub fn tick_random(now_ms: u64) -> bool {
    let Some(m) = mode() else {
        return false;
    };
    let last = ROTATED_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) < ROTATE_MS {
        return false;
    }
    set_theme(random_pick(current_id(), now_ms, m));
    ROTATED_MS.store(now_ms, Ordering::Relaxed);
    true
}

/// Advance the `Ctrl+Shift+L` cycle one step through [`THEME_MODES`]:
/// dark → light → crt → dark, wrapping. Any other state (a pinned palette or
/// the unlisted `auto`) enters at `dark`. Returns the status-line label.
pub fn cycle_next(now_ms: u64) -> &'static str {
    let next = match mode() {
        Some(RandomMode::Dark) => RandomMode::Light,
        Some(RandomMode::Light) => RandomMode::Crt,
        Some(RandomMode::Crt) => RandomMode::Dark,
        _ => RandomMode::Dark,
    };
    apply_selection(Selection::Mode(next), now_ms);
    next.as_str()
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
