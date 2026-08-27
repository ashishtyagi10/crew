//! Crew's color themes. A single `Theme` struct holds every UI colour, and
//! nine `&'static` presets fill it across four families: paper (the e-ink
//! reader look crew started as), sepia, the modern Gemini/Codex glow, and
//! three phosphor tubes. The active theme lives behind a lock-free `AtomicU8`
//! so the winit render thread can read it every frame without blocking. No
//! dependencies and no knowledge of the other crates — they import this one.
//!
//! Most of a palette is DERIVED rather than picked: the text ladder by
//! [`ramp`], the terminal slots by [`ansi`], the search wash by [`highlight`],
//! the attention colour by [`signal`]. Each of those modules holds the
//! shipped presets to what it produces, so the palettes and the system cannot
//! drift apart — which is exactly what they had done every time one of those
//! modules had to be written.
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::sync::Mutex;

mod fonts;
pub use fonts::{font_prefs, EMBEDDED_FAMILY, FONT_ALLOWLIST};

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
    /// Rounded pane border stroke width, in physical pixels. 2.5 on the dark
    /// newsprint pages, 3.0 on the light ones (a thin stroke disappears into
    /// paper), and 3.5 on the tubes and the modern family, where the frame is
    /// part of the look rather than just an edge.
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
    /// rotation pool this palette belongs to, the body text weight (light
    /// pages get a heavier stem), the CRT pass's inversion, and the
    /// light/dark scheme crew reports to DECSET-2031 terminals.
    pub dark: bool,
    /// Grain amplitude multiplier for the paper-texture pass, relative to the
    /// user's configured `paper_grain`. 1.2 on every newsprint page, dark and
    /// light alike — gamma-space blending (v0.5.58) modulates encoded values
    /// and reads far stronger than the old linear-space grain, and the
    /// shader's dark absolute term carries the texture on a dark page without
    /// a separate multiplier. The modern family is the deliberate exception at
    /// 0.0: its pages are glass with a dot lattice, not newsprint.
    /// `grain_is_newsprint_on_every_theme` is the arbiter.
    pub grain: f32,
    /// The theme's CRT tube tuning. When `Some` — and unless the user
    /// overrides it with `/crt off` — the renderer wraps the frame in the CRT
    /// post-process (curvature, scanlines, phosphor bloom, corner darkening)
    /// using these knobs. The `CRT_*` presets carry a full tube personality;
    /// the MODERN presets carry one too, but with every retro knob at zero so
    /// only the bloom runs; every paper theme is `None` so the crisp flat
    /// look is the default.
    pub crt: Option<CrtStyle>,
    /// The theme's modern-family tuning (gradient light-ring poles, drift).
    /// `Some` marks the palette as a member of the MODERN pool — the
    /// Gemini/Codex-app look — and drives the focused frame's gradient ring
    /// in crew-app. Paper and CRT presets are `None`.
    pub modern: Option<ModernStyle>,
}

pub mod ansi;
pub mod contrast;
mod crtstyle;
pub mod deco;
mod glass;
pub mod gradients;
pub mod highlight;
mod modernstyle;
pub mod oklch;
pub mod poleshift;
mod presets_crt;
mod presets_crt_cool;
mod presets_crt_violet;
mod presets_fern;
mod presets_harbor;
mod presets_modern;
mod presets_modern_light;
mod presets_paper;
mod presets_paper_light;
pub mod ramp;
pub mod readable;
pub mod signal;
mod tagcolor;
pub use crtstyle::CrtStyle;
pub use glass::{style as glass_style, style_for as glass_style_for, GlassLevel, GlassStyle};
pub use modernstyle::ModernStyle;

impl Theme {
    /// Whether this palette is a phosphor tube. Scanlines are the tell: every theme carries a
    /// [`CrtStyle`] now (the bloom chain draws the gradient ring's halo), so "has one" says
    /// nothing, and a glowing paper theme sets them to zero.
    ///
    /// Lives here rather than only on [`ThemeId`] because callers that hold a `Theme` used to
    /// re-derive it — `ansi.rs` carried its own copy of the old rule and silently disagreed the
    /// moment the rule changed.
    pub fn is_tube(&self) -> bool {
        self.crt.is_some_and(|c| c.scanline > 0.0)
    }
}

pub use presets_crt::{CRT_AMBER, CRT_GREEN};
pub use presets_crt_cool::CRT_BLUE;
pub use presets_crt_violet::CRT_VIOLET;
pub use presets_fern::FERN;
pub use presets_harbor::HARBOR;
pub use presets_modern::NEBULA;
pub use presets_modern_light::BLOSSOM;
pub use presets_paper::{PAPER_DARK, PAPER_LIGHT, SEPIA_DARK};
pub use presets_paper_light::SEPIA_LIGHT;
pub use tagcolor::{slot_color, tag_color, tag_slot};

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
    CrtGreen,
    CrtAmber,
    CrtBlue,
    Nebula,
    Blossom,
    Harbor,
    Fern,
    CrtViolet,
}

/// Every theme, in cycle order (used by the `Ctrl+Shift+L` rotation and the
/// `/theme` completion). Keep in sync with the enum.
pub const ALL_THEMES: [ThemeId; 12] = [
    ThemeId::PaperDark,
    ThemeId::PaperLight,
    ThemeId::SepiaDark,
    ThemeId::SepiaLight,
    ThemeId::Nebula,
    ThemeId::Blossom,
    ThemeId::Harbor,
    ThemeId::Fern,
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
            ThemeId::CrtGreen => "crt-green",
            ThemeId::CrtAmber => "crt-amber",
            ThemeId::CrtBlue => "crt-blue",
            ThemeId::Nebula => "nebula",
            ThemeId::Blossom => "blossom",
            ThemeId::Harbor => "harbor",
            ThemeId::Fern => "fern",
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
            ThemeId::CrtGreen => "neon green phosphor CRT",
            ThemeId::CrtAmber => "neon amber phosphor CRT",
            ThemeId::CrtBlue => "neon blue phosphor CRT (Tron)",
            ThemeId::Nebula => "orchid\u{2192}rose gradient dusk (modern dark)",
            ThemeId::Blossom => "violet\u{2192}rose on warm white (modern light)",
            ThemeId::Harbor => "blue-slate page under an azure light (dark)",
            ThemeId::Fern => "faint mint page under a green-teal light (light)",
            ThemeId::CrtViolet => "violet phosphor CRT (vector-display glow)",
        }
    }

    /// Whether this theme is dark — see [`Theme::dark`].
    pub fn is_dark(self) -> bool {
        self.theme().dark
    }

    /// Whether this palette is a PHOSPHOR TUBE — the `crt` rotation's members.
    ///
    /// Carrying a [`CrtStyle`] is not enough: every theme now carries one, because the bloom
    /// chain is how the gradient ring's halo is drawn. What makes a tube a tube is the retro
    /// knobs actually being turned up, and SCANLINES are the one no non-tube ever wants — a
    /// glowing paper theme sets them to zero.
    ///
    /// This used to read `crt.is_some() && modern.is_none()`, which was true while exactly two
    /// themes had a gradient. Once every theme has one that test says "nothing is a tube", so
    /// the question is now asked of the thing that actually differs.
    pub fn is_crt(self) -> bool {
        self.theme().is_tube()
    }

    pub fn from_name(s: &str) -> Option<ThemeId> {
        match s.trim() {
            "paper-dark" => Some(ThemeId::PaperDark),
            "paper-light" => Some(ThemeId::PaperLight),
            "sepia-dark" => Some(ThemeId::SepiaDark),
            "sepia-light" => Some(ThemeId::SepiaLight),
            "crt-green" => Some(ThemeId::CrtGreen),
            "crt-amber" => Some(ThemeId::CrtAmber),
            "crt-blue" => Some(ThemeId::CrtBlue),
            // RETIRED (2026-08-22): the roster went from 24 to 9 because
            // several palettes were a hue rotation of each other — the closest
            // pair measured Δ 0.0209, well under the Δ 0.027 at which two
            // greys stop being separable. A saved config naming one of these
            // resolves to its nearest surviving relative rather than silently
            // resetting to the default, and stays in the same family where the
            // family survived. Same courtesy `parse_selection` already extends
            // to the retired `modern` pool names.
            "midnight-ink" => Some(ThemeId::Nebula),
            "graphite" => Some(ThemeId::PaperDark),
            "moss-blotter" => Some(ThemeId::SepiaDark),
            "coldpress-gray" => Some(ThemeId::PaperLight),
            "salmon-broadsheet" => Some(ThemeId::PaperLight),
            "ivory-ledger" => Some(ThemeId::PaperLight),
            "glacier-bond" => Some(ThemeId::PaperLight),
            "crt-paperwhite" => Some(ThemeId::CrtBlue),
            "aurora" => Some(ThemeId::Nebula),
            "graphene" => Some(ThemeId::Nebula),
            "cobalt" => Some(ThemeId::Nebula),
            "daybreak" => Some(ThemeId::Blossom),
            "meadow" => Some(ThemeId::Blossom),
            "cirrus" => Some(ThemeId::Blossom),
            "nebula" => Some(ThemeId::Nebula),
            "harbor" => Some(ThemeId::Harbor),
            "harbour" => Some(ThemeId::Harbor),
            "fern" => Some(ThemeId::Fern),
            "crt-violet" => Some(ThemeId::CrtViolet),
            "crt-purple" => Some(ThemeId::CrtViolet),
            "blossom" => Some(ThemeId::Blossom),
            _ => None,
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::PaperDark => &PAPER_DARK,
            ThemeId::PaperLight => &PAPER_LIGHT,
            ThemeId::SepiaDark => &SEPIA_DARK,
            ThemeId::SepiaLight => &SEPIA_LIGHT,
            ThemeId::CrtGreen => &CRT_GREEN,
            ThemeId::CrtAmber => &CRT_AMBER,
            ThemeId::CrtBlue => &CRT_BLUE,
            ThemeId::Nebula => &NEBULA,
            ThemeId::Harbor => &HARBOR,
            ThemeId::Fern => &FERN,
            ThemeId::CrtViolet => &CRT_VIOLET,
            ThemeId::Blossom => &BLOSSOM,
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            ThemeId::PaperDark => 0,
            ThemeId::PaperLight => 1,
            ThemeId::SepiaDark => 2,
            ThemeId::SepiaLight => 3,
            ThemeId::Nebula => 4,
            ThemeId::Blossom => 5,
            ThemeId::CrtGreen => 6,
            ThemeId::CrtAmber => 7,
            ThemeId::CrtBlue => 8,
            ThemeId::Harbor => 9,
            ThemeId::Fern => 10,
            ThemeId::CrtViolet => 11,
        }
    }

    fn from_u8(v: u8) -> ThemeId {
        match v {
            1 => ThemeId::PaperLight,
            2 => ThemeId::SepiaDark,
            3 => ThemeId::SepiaLight,
            4 => ThemeId::Nebula,
            5 => ThemeId::Blossom,
            6 => ThemeId::CrtGreen,
            7 => ThemeId::CrtAmber,
            8 => ThemeId::CrtBlue,
            9 => ThemeId::Harbor,
            10 => ThemeId::Fern,
            11 => ThemeId::CrtViolet,
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
/// 0 = off, 1 = dark pool, 2 = light pool, 3 = auto (pool follows
/// [`auto_dark`]).
static MODE: AtomicU8 = AtomicU8::new(0);
/// Wall-clock ms of the last rotation (or of enabling a mode).
static ROTATED_MS: AtomicU64 = AtomicU64::new(0);
/// The OS appearance, fed by winit's ThemeChanged. Defaults to dark so a
/// platform that never reports stays on the dark pool.
static OS_DARK: AtomicBool = AtomicBool::new(true);
/// Whether the OS switches its own appearance between light and dark (macOS
/// System Settings → Appearance → Auto). Defaults to `true` — on a platform
/// crew cannot ask, the OS appearance is taken as authoritative and nothing
/// about `auto` changes. It matters only for the `false` case: an appearance
/// PINNED to dark never turns light, so following it would make `auto` a
/// synonym for `dark` forever, which is exactly the "auto is stuck on dark in
/// broad daylight" report this flag exists to answer.
static OS_AUTO: AtomicBool = AtomicBool::new(true);
/// Whether the local clock currently reads as daytime (the app's light-hours
/// window — see `daylight` in crew-app). Consulted ONLY while `OS_AUTO` is
/// false. Defaults to false so an unfed clock keeps the historical dark bias.
static DAYLIGHT: AtomicBool = AtomicBool::new(false);
/// How long each rotated theme is shown: 10 minutes (fonts share this).
pub const ROTATE_MS: u64 = 600_000;

/// A rotating theme: each mode owns a pool of palettes and cycles through them
/// every [`ROTATE_MS`]. These ARE crew's themes now — the individual palettes
/// (`PAPER_DARK`, `CRT_GREEN`, …) are the pool members, no longer offered on
/// their own. There are exactly THREE pools — `dark`, `light` and `crt` — and
/// a palette's own appearance decides which it is in: the modern (Gemini /
/// Codex look) palettes are dark and light pages like any other and rotate
/// inside `dark` / `light` rather than standing apart as their own themes.
/// `Auto` borrows the dark or light pool depending on the appearance
/// [`auto_dark`] resolves — the OS's while it self-switches, the local clock's
/// once it is pinned; [`THEME_MODES`] is the list the picker advertises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomMode {
    Dark,
    Light,
    Crt,
    Auto,
}

/// The themes crew offers: three pools plus `auto`, which serves one of them
/// per OS appearance. This is the whole user-facing theme list (`/theme`, the
/// settings picker, the `Ctrl+Shift+L` cycle); everything else (legacy
/// `random-*` / `modern*` names, individual palettes) parses for back-compat
/// but isn't advertised.
pub const THEME_MODES: [RandomMode; 4] = [
    RandomMode::Dark,
    RandomMode::Light,
    RandomMode::Crt,
    RandomMode::Auto,
];

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
            RandomMode::Dark => "rotating dark pages \u{2014} paper and modern glow",
            RandomMode::Light => "rotating light pages \u{2014} paper and modern glow",
            RandomMode::Crt => "rotating CRT phosphor themes",
            RandomMode::Auto => {
                "light by day, dark by night \u{2014} OS appearance, or the clock when it is pinned"
            }
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
    /// in exactly ONE of Dark/Light/Crt, decided by two questions in order:
    /// is it a phosphor tube ([`ThemeId::is_crt`] — which the modern family's
    /// bloom-only `CrtStyle` deliberately does not make it), and if not, is
    /// its page dark or light. So each pool is "every palette that looks like
    /// this", modern glow and plain paper alike, and a rotation can never flip
    /// the page from near-black to near-white. `Auto` serves its
    /// per-appearance pairing ([`auto_side`]) — by default the dark or light
    /// pool depending on [`auto_dark`], a pinned side being a one-palette
    /// pool.
    pub fn in_pool(self, id: ThemeId) -> bool {
        match self {
            RandomMode::Dark => id.is_dark() && !id.is_crt(),
            RandomMode::Light => !id.is_dark() && !id.is_crt(),
            RandomMode::Crt => id.is_crt(),
            RandomMode::Auto => match auto_side() {
                Selection::Mode(m) => m.in_pool(id),
                Selection::Fixed(f) => id == f,
            },
        }
    }
}

/// What a theme name string resolves to: a fixed theme or a rotation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Fixed(ThemeId),
    Mode(RandomMode),
}

impl Selection {
    /// The name this selection round-trips through [`parse_selection`] — the
    /// mode's name while rotating, else the pinned palette's.
    pub fn label(self) -> &'static str {
        match self {
            Selection::Fixed(id) => id.as_str(),
            Selection::Mode(m) => m.as_str(),
        }
    }
}

/// Parse a `/theme` argument / config value. The four canonical names are
/// `dark`, `light`, `crt`, `auto`; the pre-consolidation names (`random`,
/// `random-dark`, `random-light`, and the `modern` / `modern-light` modes
/// that are now simply part of `dark` / `light`) and every individual palette
/// name still parse so old configs keep loading — a name that stops parsing is
/// a config line that silently does nothing, which is exactly the failure this
/// list exists to prevent.
pub fn parse_selection(s: &str) -> Option<Selection> {
    let s = s.trim();
    // `modern` was a dark-page rotation and `modern-light` a light-page one;
    // both pools were folded into `dark` / `light`, so the old names resolve
    // to the pool that swallowed them (a superset — the same palettes still
    // come up, alongside the paper ones).
    if s.eq_ignore_ascii_case("dark")
        || s.eq_ignore_ascii_case("random")
        || s.eq_ignore_ascii_case("random-dark")
        || s.eq_ignore_ascii_case("modern")
        || s.eq_ignore_ascii_case("random-modern")
    {
        return Some(Selection::Mode(RandomMode::Dark));
    }
    if s.eq_ignore_ascii_case("light")
        || s.eq_ignore_ascii_case("random-light")
        || s.eq_ignore_ascii_case("modern-light")
        || s.eq_ignore_ascii_case("modern light")
        || s.eq_ignore_ascii_case("modernlight")
        || s.eq_ignore_ascii_case("random-modern-light")
    {
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

/// Report whether the OS switches its OWN appearance on a schedule (macOS
/// Appearance: Auto). Fed at startup and on every ThemeChanged.
pub fn set_os_auto(auto: bool) {
    OS_AUTO.store(auto, Ordering::Relaxed);
}

/// Whether the OS appearance is self-switching (defaults to `true`: assume
/// the OS is authoritative unless crew has been told otherwise).
pub fn os_auto() -> bool {
    OS_AUTO.load(Ordering::Relaxed)
}

/// Report whether the local clock currently reads as daytime.
pub fn set_daylight(day: bool) {
    DAYLIGHT.store(day, Ordering::Relaxed);
}

/// The last reported daylight state (defaults to false = night).
pub fn daylight() -> bool {
    DAYLIGHT.load(Ordering::Relaxed)
}

/// Which half `auto` should serve, as a single yes/no — the ONE place the
/// two clocks are weighed against each other, so every caller (pool
/// membership, the `/theme` report, the settings blurb) agrees.
///
/// While the OS switches itself, the OS wins: it already encodes the user's
/// day/night preference, schedule and all. Once the appearance is PINNED,
/// following it makes `auto` indistinguishable from `dark` (or `light`)
/// forever, so crew falls back to its own light-hours window — which is what
/// "light by day, dark by night" says on the tin.
pub fn auto_dark() -> bool {
    if os_auto() {
        os_dark()
    } else {
        !daylight()
    }
}

/// What `auto` serves per OS appearance: `.0` while dark, `.1` while light.
/// `None` = the built-in pairing (the dark/light paper pool). Read on theme
/// application and rotation ticks, not per frame, so a mutex is fine.
static AUTO_POOLS: Mutex<(Option<Selection>, Option<Selection>)> = Mutex::new((None, None));

/// Configure `auto`'s per-appearance pairing (config `theme_dark` /
/// `theme_light`): each side is a rotation pool (`dark`|`light`|`crt`) or a
/// pinned palette — e.g. phosphor tubes at night, light paper by day. A side
/// of `auto` itself would recurse and is dropped to the default; `None`
/// keeps the built-in paper pool for that appearance.
pub fn set_auto_pools(dark: Option<Selection>, light: Option<Selection>) {
    let clean = |s: Option<Selection>| s.filter(|s| *s != Selection::Mode(RandomMode::Auto));
    *AUTO_POOLS.lock().unwrap() = (clean(dark), clean(light));
}

/// The configured pairing itself: `.0` for the dark appearance, `.1` for the
/// light one, `None` on a side meaning "the built-in paper pool". Read it to
/// TELL the user what `auto` is holding — a side paired for the appearance
/// they are not currently in has no other symptom (`theme_light =
/// "modern-light"` under a dark OS looks exactly like a setting that does
/// nothing at all).
pub fn auto_pools() -> (Option<Selection>, Option<Selection>) {
    *AUTO_POOLS.lock().unwrap()
}

/// `auto`'s light-hours window as minutes past midnight (`.0` start, `.1`
/// end), for REPORTING only — the app owns the window, parses it from config
/// and decides [`set_daylight`] from it. It lives here beside [`AUTO_POOLS`]
/// for the same reason that does: `/theme` has to be able to read back every
/// part of what `auto` is holding, and a window it cannot name is a setting
/// with no visible effect.
static LIGHT_HOURS: Mutex<(u16, u16)> = Mutex::new((7 * 60, 19 * 60));

/// Publish `auto`'s configured light-hours window.
pub fn set_light_hours(from: u16, to: u16) {
    *LIGHT_HOURS.lock().unwrap() = (from, to);
}

/// The configured light-hours window (minutes past midnight).
pub fn light_hours() -> (u16, u16) {
    *LIGHT_HOURS.lock().unwrap()
}

/// The selection `auto` resolves to under the current appearance. Never
/// `Mode(Auto)` (see [`set_auto_pools`]), so pool membership can't recurse.
pub fn auto_side() -> Selection {
    let (dark, light) = *AUTO_POOLS.lock().unwrap();
    if auto_dark() {
        dark.unwrap_or(Selection::Mode(RandomMode::Dark))
    } else {
        light.unwrap_or(Selection::Mode(RandomMode::Light))
    }
}

/// Pick a theme from `mode`'s pool that is NOT `current`, deterministically
/// from `seed`. Every pool has exactly 3 entries since the nine-theme cut
/// (`every_pool_survives_the_cut` is where that is pinned), so minus `current`
/// it is never empty; the `current` filter is skipped only if a future cut
/// WOULD empty the pool, which keeps the modulo safe rather than relying on a
/// roster size to stay where it is.
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
/// dark → light → crt → auto → dark, wrapping. Any other state (a pinned
/// palette) enters at `dark`. The order IS `THEME_MODES` — walking the list
/// rather than hand-writing the successors is what keeps a newly added mode
/// from being silently unreachable by the hotkey (which is exactly what
/// happened when a fourth pool was added). Returns the status-line label.
pub fn cycle_next(now_ms: u64) -> &'static str {
    let next = match mode() {
        Some(m) => {
            let i = THEME_MODES.iter().position(|&x| x == m).unwrap_or(0);
            THEME_MODES[(i + 1) % THEME_MODES.len()]
        }
        None => RandomMode::Dark,
    };
    apply_selection(Selection::Mode(next), now_ms);
    next.as_str()
}

/// The lock every test that writes a process-wide theme global takes —
/// the selection, the OS appearance, the pole shift. One lock for the whole
/// crate rather than one per test file, because the globals are shared: two
/// files with two locks would serialise against themselves and race each
/// other, which is worse than no lock at all for being harder to see.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
