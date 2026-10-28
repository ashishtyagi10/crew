//! Crew's color themes. A single `Theme` struct holds every UI colour, and
//! fifteen `&'static` presets fill it: the Ink Reader pages, the smoked
//! glass, two phosphor tubes, neon, five classic editor palettes and a purple one. The active theme lives behind a lock-free `AtomicU8`
//! so the winit render thread can read it every frame without blocking. No
//! dependencies and no knowledge of the other crates — they import this one.
//!
//! Most of a palette is DERIVED rather than picked: the text ladder by
//! [`ramp`], the terminal slots by [`ansi`], the search wash by [`highlight`],
//! the attention colour by [`signal`]. Each of those modules holds the
//! shipped presets to what it produces, so the palettes and the system cannot
//! drift apart — which is exactly what they had done every time one of those
//! modules had to be written.
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};

mod favorite;
mod fonts;
pub use favorite::{is_favorite, spelling_rank, typeface_key, FAVORITES, FAVORITE_WEIGHT};
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
    /// Rounded pane border stroke width, in physical pixels. 2.5 on the Ink
    /// Reader pages, where a line is drawn the way print draws it, 2.0 on the
    /// clear glass, and 3.5 on the tubes and the modern family, where the
    /// frame is part of the look rather than just an edge.
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
    /// The theme's CRT tube tuning. When `Some` — and unless the user
    /// overrides it with `/crt off` — the renderer wraps the frame in the CRT
    /// post-process (curvature, scanlines, phosphor bloom, corner darkening)
    /// using these knobs. The `CRT_*` presets carry a full tube personality;
    /// the MODERN presets carry one too, but with every retro knob at zero so
    /// only the bloom runs; every paper theme is `None` so the crisp flat
    /// look is the default.
    pub crt: Option<CrtStyle>,
    /// The gradient light-ring's poles and drift period: the focused
    /// frame's ring in crew-app. `Some` on every palette.
    pub modern: Option<ModernStyle>,
    /// Liquid glass: the window and its panes let the desktop through
    /// ([`LiquidStyle`]). `Some` on every palette since 2026-10-09.
    pub liquid: Option<LiquidStyle>,
}

pub mod ansi;
pub mod contrast;
mod crtstyle;
pub mod deco;
mod glass;
pub mod glassborder;
pub mod glassink;
pub mod glasslegend;
pub mod gradients;
pub mod highlight;
pub mod holo;
mod liquid;
pub mod marks;
mod modernstyle;
pub mod oklch;
pub mod poleshift;
mod presets_classic;
mod presets_classic2;
mod presets_crt;
mod presets_glass;
mod presets_glass_clear;
mod presets_modern;
mod presets_modern_light;
mod presets_neon;
mod presets_paper;
mod presets_purple;
pub mod ramp;
pub mod readable;
pub mod signal;
mod tagcolor;
pub use crtstyle::CrtStyle;
pub use glass::{
    style as glass_style, style_at as glass_style_at, style_for as glass_style_for, GlassLevel,
    GlassStyle,
};
pub use liquid::LiquidStyle;
pub use modernstyle::ModernStyle;

impl Theme {
    /// Whether this palette is a phosphor tube. The FILAMENT is the tell ([`CrtStyle::core`]):
    /// every theme carries a [`CrtStyle`] (the bloom chain draws the gradient ring's halo), so
    /// "has one" says nothing, and only a tube's strokes burn white-hot. Scanlines were the tell
    /// until the glass-tube goal (2026-10-05) moved a tube's raster off the window and into its
    /// glass ([`CrtStyle::etch`]), leaving every tube's scanlines at zero.
    ///
    /// Lives here rather than only on [`ThemeId`] because callers that hold a `Theme` used to
    /// re-derive it — `ansi.rs` carried its own copy of the old rule and silently disagreed the
    /// moment the rule changed.
    pub fn is_tube(&self) -> bool {
        self.crt.is_some_and(|c| c.core > 0.0)
    }

    /// Whether this palette is an Ink Reader page (the 2026-10-16 goal): print, with no glass
    /// and no tube — what [`glass_style_for`] draws flat.
    pub fn is_print(&self) -> bool {
        !self.is_tube() && self.liquid.is_none()
    }
}

pub use presets_classic::{SOLARIZED, TOKYO_NIGHT};
pub use presets_classic2::{EVERFOREST, GRUVBOX, MONOKAI};
pub use presets_crt::{CRT_AMBER, CRT_GREEN};
pub use presets_glass::GLASS_NIGHT;
pub use presets_glass_clear::GLASS_CLEAR;
pub use presets_modern::NEBULA;
pub use presets_modern_light::BLOSSOM;
pub use presets_neon::NEON_DARK;
pub use presets_paper::{PAPER_DARK, PAPER_LIGHT};
pub use presets_purple::AMETHYST;
pub use tagcolor::{slot_color, tag_color, tag_slot};

/// WCAG 2.1 relative luminance of an sRGB colour — how much light it
/// actually emits, which is not what its bytes look like. The basis of
/// [`contrast_ratio`], and of which way the renderer's text-gamma curve
/// bends for a given pair of colours.
pub fn relative_luminance(c: (u8, u8, u8)) -> f32 {
    let lin = |c: u8| -> f32 {
        let x = c as f32 / 255.0;
        if x <= 0.03928 {
            x / 12.92
        } else {
            ((x + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(c.0) + 0.7152 * lin(c.1) + 0.0722 * lin(c.2)
}

/// WCAG 2.1 contrast ratio between two sRGB colours.
pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f32 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeId {
    PaperDark,
    PaperLight,
    CrtGreen,
    CrtAmber,
    Nebula,
    Blossom,
    GlassNight,
    GlassClear,
    NeonDark,
    TokyoNight,
    Solarized,
    Everforest,
    Gruvbox,
    Monokai,
    Amethyst,
}

/// Every theme, in cycle order (used by the `Ctrl+Shift+L` rotation and the
/// `/theme` completion). Keep in sync with the enum.
pub const ALL_THEMES: [ThemeId; 15] = [
    ThemeId::PaperDark,
    ThemeId::PaperLight,
    ThemeId::Nebula,
    ThemeId::Blossom,
    ThemeId::GlassClear,
    ThemeId::GlassNight,
    ThemeId::CrtGreen,
    ThemeId::CrtAmber,
    ThemeId::NeonDark,
    ThemeId::TokyoNight,
    ThemeId::Solarized,
    ThemeId::Everforest,
    ThemeId::Gruvbox,
    ThemeId::Monokai,
    ThemeId::Amethyst,
];

impl ThemeId {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "paper-dark",
            ThemeId::PaperLight => "paper-light",
            ThemeId::CrtGreen => "crt-green",
            ThemeId::CrtAmber => "crt-amber",
            ThemeId::Nebula => "nebula",
            ThemeId::Blossom => "blossom",
            ThemeId::GlassNight => "glass-night",
            ThemeId::GlassClear => "glass-clear",
            ThemeId::NeonDark => "neon-dark",
            ThemeId::TokyoNight => "tokyo-night",
            ThemeId::Solarized => "solarized",
            ThemeId::Everforest => "everforest",
            ThemeId::Gruvbox => "gruvbox",
            ThemeId::Monokai => "monokai",
            ThemeId::Amethyst => "amethyst",
        }
    }

    /// A short human description, for the `/theme` value picker.
    pub fn describe(self) -> &'static str {
        match self {
            ThemeId::PaperDark => "black page, white words: the Ink Reader (dark)",
            ThemeId::PaperLight => "white page, black words: the Ink Reader (light)",
            ThemeId::CrtGreen => "green phosphor CRT",
            ThemeId::CrtAmber => "amber phosphor CRT",
            ThemeId::Nebula => "violet smoked glass, orchid\u{2192}rose ring, white text",
            ThemeId::Blossom => "cherry-blossom smoked glass, pink accents, white text",
            ThemeId::GlassNight => "navy-smoked clear glass, white text (iPhone liquid glass)",
            ThemeId::GlassClear => "lightly smoked clear glass, white text (iPhone liquid glass)",
            ThemeId::NeonDark => "holographic glass over a lit grid",
            ThemeId::TokyoNight => "Tokyo Night: deep navy, cyan accent, blue\u{2192}magenta ring",
            ThemeId::Solarized => "Solarized: deep teal, yellow accent, blue\u{2192}cyan ring",
            ThemeId::Everforest => {
                "Everforest: forest green-grey, parchment text, green\u{2192}aqua ring"
            }
            ThemeId::Gruvbox => "Gruvbox: retro charcoal, cream text, yellow\u{2192}orange ring",
            ThemeId::Monokai => "Monokai: olive-black, green accent, pink\u{2192}purple ring",
            ThemeId::Amethyst => {
                "Amethyst: royal-purple glass, lavender accent, violet\u{2192}magenta ring"
            }
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

    /// The rotation this palette belongs to — exactly one of `dark`,
    /// `light`, `crt`, `glass`, `neon`, `classic` and `purple`. Named per palette. `dark` and `light` are
    /// the Ink Reader pages alone (the user, 2026-10-16: the light and dark
    /// themes "Ink Reader like"): a light mode that rotated to a dark glass
    /// palette every ten minutes would not be one. Nebula and blossom joined
    /// the glass they are.
    pub fn mode(self) -> RandomMode {
        match self {
            ThemeId::PaperDark => RandomMode::Dark,
            ThemeId::PaperLight => RandomMode::Light,
            ThemeId::CrtGreen | ThemeId::CrtAmber => RandomMode::Crt,
            ThemeId::GlassClear | ThemeId::GlassNight | ThemeId::Nebula | ThemeId::Blossom => {
                RandomMode::Glass
            }
            ThemeId::NeonDark => RandomMode::Neon,
            ThemeId::TokyoNight
            | ThemeId::Solarized
            | ThemeId::Everforest
            | ThemeId::Gruvbox
            | ThemeId::Monokai => RandomMode::Classic,
            ThemeId::Amethyst => RandomMode::Purple,
        }
    }

    /// What naming this palette selects. A palette that is its rotation's
    /// whole pool IS that rotation, so it resolves to the rotation:
    /// `paper-dark` is `dark`, `paper-light` is `light`, `neon-dark` is
    /// `neon` and `amethyst` is `purple`, and the pickers list each theme once (the user, 2026-10-22: "remove the duplicate
    /// themes … dark == paper-dark, light == paper-light"). Every other
    /// palette pins itself.
    pub fn selection(self) -> Selection {
        let m = self.mode();
        match m.sole_palette() {
            Some(_) => Selection::Mode(m),
            None => Selection::Fixed(self),
        }
    }

    /// Whether this palette is WHITE-TEXT liquid glass — smoked panes over
    /// the desktop and white words: the `glass` rotation, and the `dark` one
    /// since it went see-through (the user, 2026-10-09: "bright white would
    /// look great on glass").
    pub fn is_clear_glass(self) -> bool {
        let t = self.theme();
        t.liquid.is_some() && t.dark
    }

    pub fn from_name(s: &str) -> Option<ThemeId> {
        match s.trim() {
            "paper-dark" => Some(ThemeId::PaperDark),
            "paper-light" => Some(ThemeId::PaperLight),
            // Folded into the dark glass (2026-10-09: two palettes a mode).
            "sepia-dark" | "harbor" | "harbour" => Some(ThemeId::PaperDark),
            "sepia-light" => Some(ThemeId::PaperLight),
            "crt-green" => Some(ThemeId::CrtGreen),
            "crt-amber" => Some(ThemeId::CrtAmber),
            // Two tubes since every mode went see-through (2026-10-09).
            "crt-blue" => Some(ThemeId::CrtGreen),
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
            "moss-blotter" => Some(ThemeId::PaperDark),
            "coldpress-gray" => Some(ThemeId::PaperLight),
            "salmon-broadsheet" => Some(ThemeId::PaperLight),
            "ivory-ledger" => Some(ThemeId::PaperLight),
            "glacier-bond" => Some(ThemeId::PaperLight),
            "crt-paperwhite" => Some(ThemeId::CrtGreen),
            "aurora" => Some(ThemeId::Nebula),
            "graphene" => Some(ThemeId::Nebula),
            "cobalt" => Some(ThemeId::Nebula),
            "daybreak" => Some(ThemeId::Blossom),
            "meadow" => Some(ThemeId::Blossom),
            "cirrus" => Some(ThemeId::Blossom),
            "nebula" => Some(ThemeId::Nebula),
            "fern" => Some(ThemeId::Blossom),
            "crt-violet" | "crt-purple" => Some(ThemeId::CrtAmber),
            "blossom" => Some(ThemeId::Blossom),
            // `glass` is a rotation (`parse_selection` catches it first); a
            // caller asking for one palette by that name gets the sky.
            "glass" | "glass-clear" => Some(ThemeId::GlassClear),
            // The frosted light pair folded into the clear glass (the user,
            // 2026-10-09: "glass-sky and glass-dusk … should also be clear").
            "glass-light" | "glass-sky" | "glass-dawn" => Some(ThemeId::GlassClear),
            "glass-night" | "glass-dark" => Some(ThemeId::GlassNight),
            // Neon had a light page until 2026-10-22; a config that saved
            // it gets the one neon there is.
            "neon-dark" | "neon-light" => Some(ThemeId::NeonDark),
            "tokyo-night" => Some(ThemeId::TokyoNight),
            "solarized" => Some(ThemeId::Solarized),
            "everforest" => Some(ThemeId::Everforest),
            "gruvbox" => Some(ThemeId::Gruvbox),
            "monokai" => Some(ThemeId::Monokai),
            "amethyst" => Some(ThemeId::Amethyst),
            _ => None,
        }
    }

    pub fn theme(self) -> &'static Theme {
        match self {
            ThemeId::PaperDark => &PAPER_DARK,
            ThemeId::PaperLight => &PAPER_LIGHT,
            ThemeId::CrtGreen => &CRT_GREEN,
            ThemeId::CrtAmber => &CRT_AMBER,
            ThemeId::Nebula => &NEBULA,
            ThemeId::Blossom => &BLOSSOM,
            ThemeId::GlassNight => &GLASS_NIGHT,
            ThemeId::GlassClear => &GLASS_CLEAR,
            ThemeId::NeonDark => &NEON_DARK,
            ThemeId::TokyoNight => &TOKYO_NIGHT,
            ThemeId::Solarized => &SOLARIZED,
            ThemeId::Everforest => &EVERFOREST,
            ThemeId::Gruvbox => &GRUVBOX,
            ThemeId::Monokai => &MONOKAI,
            ThemeId::Amethyst => &AMETHYST,
        }
    }

    /// Its place in [`ALL_THEMES`], for the lock-free [`CURRENT`]. The first
    /// entry is the default, so a zeroed atomic is `PaperDark`.
    fn as_u8(self) -> u8 {
        ALL_THEMES.iter().position(|&t| t == self).unwrap_or(0) as u8
    }

    fn from_u8(v: u8) -> ThemeId {
        ALL_THEMES
            .get(usize::from(v))
            .copied()
            .unwrap_or(ThemeId::PaperDark)
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
    let id = current_id();
    // A sheer window serves the palette with its frames lifted (see
    // `glassborder`); an opaque one, the palette as written.
    if glassborder::sheer() {
        glassborder::lifted(id)
    } else {
        id.theme()
    }
}

/// Rotation mode: when set, the active theme changes every [`ROTATE_MS`]
/// within the mode's pool. Stored as a lock-free u8 for per-frame reads:
/// 0 = off, else [`RandomMode::as_u8`] (3 = auto, every look in turn).
static MODE: AtomicU8 = AtomicU8::new(0);
/// Wall-clock ms of the last rotation (or of enabling a mode).
static ROTATED_MS: AtomicU64 = AtomicU64::new(0);
/// How long each rotated theme is shown: 10 minutes (fonts share this).
pub const ROTATE_MS: u64 = 600_000;

/// A rotating theme: each mode owns a pool of palettes and cycles through them
/// every [`ROTATE_MS`]. These ARE crew's themes now — the individual palettes
/// (`PAPER_DARK`, `CRT_GREEN`, …) are the pool members, no longer offered on
/// their own. There are exactly THREE pools — `dark`, `light` and `crt` — and
/// a palette's own appearance decides which it is in: the modern (Gemini /
/// Codex look) palettes are dark and light pages like any other and rotate
/// inside `dark` / `light` rather than standing apart as their own themes.
/// `Auto` walks the other looks in turn ([`auto_next`]), whatever the clock
/// or the OS appearance; [`THEME_MODES`] is the list the picker advertises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RandomMode {
    Dark,
    Light,
    Crt,
    Glass,
    /// The neon control room: one dark page of holographic glass, whatever
    /// the appearance (the user, 2026-10-22: "keep it default dark").
    Neon,
    /// The editor classics — Tokyo Night, Solarized, Everforest, Gruvbox
    /// and Monokai — as smoked glass (the user, 2026-10-28: "add 5 new
    /// theme for crew").
    Classic,
    /// One royal-purple page of smoked glass, `amethyst` (the user,
    /// 2026-10-28: "add one purple color theme").
    Purple,
    /// Every look in turn, a new one each [`ROTATE_MS`] (the user,
    /// 2026-10-25: "rotate through all available theme, irrespective of
    /// time").
    Auto,
}

/// The themes crew offers: seven pools plus `auto`, which walks them all.
/// This is the whole user-facing theme list (`/theme`, the settings picker,
/// the `Ctrl+Shift+L` cycle); everything else (legacy `random-*` / `modern*`
/// names, individual palettes) parses for back-compat but isn't advertised.
pub const THEME_MODES: [RandomMode; 8] = [
    RandomMode::Dark,
    RandomMode::Light,
    RandomMode::Crt,
    RandomMode::Glass,
    RandomMode::Neon,
    RandomMode::Classic,
    RandomMode::Purple,
    RandomMode::Auto,
];

impl RandomMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RandomMode::Dark => "dark",
            RandomMode::Light => "light",
            RandomMode::Crt => "crt",
            RandomMode::Glass => "glass",
            RandomMode::Neon => "neon",
            RandomMode::Classic => "classic",
            RandomMode::Purple => "purple",
            RandomMode::Auto => "auto",
        }
    }

    /// A short human description, for the `/theme` value picker and listings.
    pub fn describe(self) -> &'static str {
        match self {
            RandomMode::Dark => "black page, white words \u{2014} the Ink Reader, an e-reader's look",
            RandomMode::Light => "white page, black words \u{2014} the Ink Reader, an e-reader's look",
            RandomMode::Crt => "rotating CRT phosphor themes",
            RandomMode::Glass => {
                "clear smoked panes, bright white text, your desktop behind \u{2014} iPhone liquid glass"
            }
            RandomMode::Neon => "holographic glass over a lit grid, always dark",
            RandomMode::Classic => {
                "Tokyo Night, Solarized, Everforest, Gruvbox, Monokai \u{2014} the editor classics as smoked glass"
            }
            RandomMode::Purple => "amethyst: royal-purple smoked glass, lavender accent, always dark",
            RandomMode::Auto => {
                "every look in turn \u{2014} dark, light, crt, glass, neon, classic, purple \u{2014} a new one every 10 minutes"
            }
        }
    }

    fn as_u8(self) -> u8 {
        match self {
            RandomMode::Dark => 1,
            RandomMode::Light => 2,
            RandomMode::Auto => 3,
            RandomMode::Crt => 4,
            RandomMode::Glass => 5,
            RandomMode::Neon => 6,
            RandomMode::Classic => 7,
            RandomMode::Purple => 8,
        }
    }

    fn from_u8(v: u8) -> Option<RandomMode> {
        match v {
            1 => Some(RandomMode::Dark),
            2 => Some(RandomMode::Light),
            3 => Some(RandomMode::Auto),
            4 => Some(RandomMode::Crt),
            5 => Some(RandomMode::Glass),
            6 => Some(RandomMode::Neon),
            7 => Some(RandomMode::Classic),
            8 => Some(RandomMode::Purple),
            _ => None,
        }
    }

    /// The one palette this rotation serves when its pool is a single page
    /// (`dark` and `light`, the Ink Reader pages, `neon` and `purple`). `None` for a
    /// pool that changes palette, and for `auto`, which serves them all.
    pub fn sole_palette(self) -> Option<ThemeId> {
        if self == RandomMode::Auto {
            return None;
        }
        let mut pool = ALL_THEMES.into_iter().filter(|&t| t.mode() == self);
        match (pool.next(), pool.next()) {
            (Some(id), None) => Some(id),
            _ => None,
        }
    }

    /// Whether `id` belongs to this mode's rotation pool: its
    /// [`ThemeId::mode`], so every palette lands in exactly one of
    /// Dark/Light/Crt/Glass/Neon/Classic/Purple and a rotation stays inside one look. `Auto`
    /// holds every palette.
    pub fn in_pool(self, id: ThemeId) -> bool {
        self == RandomMode::Auto || id.mode() == self
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
/// list exists to prevent. `paper-dark` and `paper-light` load as the `dark`
/// and `light` they are, `neon-dark` (and the retired `neon-light`) as
/// `neon`, and `amethyst` as `purple` ([`ThemeId::selection`]).
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
    if s.eq_ignore_ascii_case("glass") || s.eq_ignore_ascii_case("liquid-glass") {
        return Some(Selection::Mode(RandomMode::Glass));
    }
    if s.eq_ignore_ascii_case("neon") {
        return Some(Selection::Mode(RandomMode::Neon));
    }
    if s.eq_ignore_ascii_case("classic") {
        return Some(Selection::Mode(RandomMode::Classic));
    }
    if s.eq_ignore_ascii_case("purple") {
        return Some(Selection::Mode(RandomMode::Purple));
    }
    if s.eq_ignore_ascii_case("auto") {
        return Some(Selection::Mode(RandomMode::Auto));
    }
    ThemeId::from_name(s).map(ThemeId::selection)
}

/// The palettes a picker offers one by one, in [`ALL_THEMES`] order: those
/// that pin themselves. The Ink Reader pages, neon's and amethyst are left
/// out because the picker already lists them as `dark`, `light`, `neon` and
/// `purple` (see
/// [`ThemeId::selection`]).
pub fn pinnable_themes() -> impl Iterator<Item = ThemeId> {
    ALL_THEMES
        .into_iter()
        .filter(|id| matches!(id.selection(), Selection::Fixed(_)))
}

/// The active rotation mode, if any.
pub fn mode() -> Option<RandomMode> {
    RandomMode::from_u8(MODE.load(Ordering::Relaxed))
}

/// Whether any rotation mode is active.
pub fn is_random() -> bool {
    mode().is_some()
}

/// The looks `auto` walks, in [`THEME_MODES`] order: every theme but
/// `auto` itself.
pub fn auto_looks() -> impl Iterator<Item = RandomMode> {
    THEME_MODES.into_iter().filter(|&m| m != RandomMode::Auto)
}

/// The look `auto` turns to after `current`'s: the next of [`auto_looks`],
/// wrapping, so every rotation changes the look and all seven come round in
/// turn. Neither the clock nor the OS appearance has a say — `auto` used to
/// serve the appearance's page, which on a Mac pinned to Dark was `dark` for
/// good.
pub fn auto_next(current: ThemeId) -> RandomMode {
    let looks: Vec<RandomMode> = auto_looks().collect();
    let i = looks.iter().position(|&m| m == current.mode()).unwrap_or(0);
    looks[(i + 1) % looks.len()]
}

/// The look switching TO `auto` opens on: any but `current`'s, from `seed`.
/// Not simply [`auto_next`], because at launch `current` is always the
/// zeroed default, and every launch would open on the same look.
fn auto_start(current: ThemeId, seed: u64) -> RandomMode {
    let others: Vec<RandomMode> = auto_looks().filter(|&m| m != current.mode()).collect();
    others[spread(seed, others.len())]
}

/// An index below `n` from `seed`, spread so that neighbouring seeds (two
/// wall-clock ms apart) land far apart.
fn spread(seed: u64, n: usize) -> usize {
    (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % n
}

/// Pick a theme from `mode`'s pool that is NOT `current`, deterministically
/// from `seed`. A one-page pool (`dark`, `light`, `neon`, `purple`) holds only
/// `current` (`every_pool_survives_the_cut` pins the sizes), so the `current`
/// filter is skipped when it WOULD empty the pool, which keeps the modulo
/// safe rather than relying on a roster size to stay where it is.
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
    others[spread(seed, others.len())]
}

/// Apply a parsed selection: a fixed theme pins it (mode off); a mode
/// switches immediately to a pick from its pool (so the effect is visible)
/// and starts the 10-minute clock. `auto` opens on a look other than the
/// one on screen ([`auto_start`]).
pub fn apply_selection(sel: Selection, now_ms: u64) {
    match sel {
        Selection::Fixed(id) => {
            MODE.store(0, Ordering::Relaxed);
            set_theme(id);
        }
        Selection::Mode(m) => {
            MODE.store(m.as_u8(), Ordering::Relaxed);
            let pool = match m {
                RandomMode::Auto => auto_start(current_id(), now_ms),
                m => m,
            };
            set_theme(random_pick(current_id(), now_ms, pool));
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
/// (auto's: the next look, [`auto_next`]) and return `true` so the caller
/// repaints. A one-page pool (the Ink Reader modes) holds its page and
/// returns `false`: nothing changed. Cheap and lock-free — safe at ~62 Hz on
/// the winit thread.
pub fn tick_random(now_ms: u64) -> bool {
    let Some(m) = mode() else {
        return false;
    };
    let last = ROTATED_MS.load(Ordering::Relaxed);
    if now_ms.saturating_sub(last) < ROTATE_MS {
        return false;
    }
    let pool = match m {
        RandomMode::Auto => auto_next(current_id()),
        m => m,
    };
    let next = random_pick(current_id(), now_ms, pool);
    ROTATED_MS.store(now_ms, Ordering::Relaxed);
    if next == current_id() {
        return false;
    }
    set_theme(next);
    true
}

/// Advance the `Ctrl+Shift+L` cycle one step through [`THEME_MODES`]:
/// dark → light → crt → glass → neon → classic → purple → auto → dark, wrapping. Any other state (a pinned
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
/// the selection, the pole shift. One lock for the whole
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
