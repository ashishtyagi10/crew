//! What each setting is when `config.toml` does not say: one `#[serde(default
//! = "...")]` function per field.
//!
//! Split from [`crate::config`] for the line cap. They are a wall of
//! three-line functions rather than logic, and they were half the file that
//! defines the config itself.

pub(crate) fn default_font_size() -> f32 {
    14.0
}

pub(crate) fn default_nav_width() -> f32 {
    210.0
}

pub(crate) fn default_show_nav() -> bool {
    true
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn default_notify_min_secs() -> u64 {
    10
}

pub(crate) fn default_usage_budget_5h() -> u64 {
    5_000_000
}

pub(crate) fn default_usage_budget_7d() -> u64 {
    25_000_000
}

pub(crate) fn default_paper_grain() -> f32 {
    // ~2.6% luminance grain — clearly reads as paper texture without looking
    // noisy (chosen by comparing a rendered 0.0/0.6/1.0/1.6 sweep). Tunable in
    // config; 0.0 disables grain, paper_texture=false disables the whole pass.
    1.3
}

pub(crate) fn default_gradient() -> String {
    // The gradient breathes by default, gently. `off` pins the poles to the
    // theme's own colours (see `gradientlvl::GradientLevel`).
    "subtle".to_string()
}

pub(crate) fn default_shape_cues() -> String {
    // Follow the OS: macOS's Accessibility -> Display -> "Differentiate
    // without color" is the switch this answers (see `shapecues`).
    "auto".to_string()
}

pub(crate) fn default_contrast() -> String {
    // Follow the OS: macOS's Accessibility -> Display -> "Increase contrast"
    // is where a user has already said this once (see `crew_theme::contrast`).
    "auto".to_string()
}

pub(crate) fn default_leading() -> String {
    // `1.25 × font_size` — the line height crew has always drawn (see
    // `leading::Leading::ratio`), so the knob arriving changes nothing until
    // someone turns it.
    "normal".to_string()
}

pub(crate) fn default_density() -> String {
    // The layout crew has always drawn (see `density::Density::gap_px`), so
    // the knob arriving changes nothing until someone turns it.
    "cozy".to_string()
}

pub(crate) fn default_motion() -> String {
    // Crew follows the OS by default: macOS's Accessibility → "Reduce motion"
    // is the system-wide way to ask for this, and an app that ignores it makes
    // the user hunt for a private setting they already set once. With the
    // switch off, `auto` is full motion — the historical default (see
    // `motion::MotionPref`). An explicit level still overrules the OS.
    "auto".to_string()
}

pub(crate) fn default_auto_light_from() -> String {
    "07:00".to_string()
}

pub(crate) fn default_auto_light_to() -> String {
    "19:00".to_string()
}

pub(crate) fn default_glass() -> String {
    // Strength only; the look is derived per-theme (see `crew_theme::glass`).
    // Since 2026-08-06 paper themes derive a flat (invisible) sheet, so this
    // dial only shows on CRT themes — `off` kills even the holographic sheet.
    "medium".to_string()
}

/// Floor for [`CrewConfig::window_opacity`]. Mirrors the renderer's own clamp:
/// a window dialled to invisible is a window the user cannot get back.
pub const MIN_WINDOW_OPACITY: f32 = 0.35;

pub(crate) fn default_window_opacity() -> f32 {
    // Opaque. Window translucency is opt-in via Settings → WINDOW → Opacity % — a
    // see-through terminal is a taste, not a default.
    1.0
}

pub(crate) fn default_font_weight() -> u16 {
    // SemiBold. Heavier than the old Medium (500) base so body text reads
    // thicker and more substantial out of the box; /weight tunes it live.
    600
}

pub(crate) fn default_font_smooth() -> u8 {
    // The renderer's calibrated CoreText-style stem darkening.
    crew_render::DEFAULT_SMOOTH
}

pub(crate) fn default_font_gamma() -> u8 {
    // About half the full sRGB correction — Apple's historical text gamma.
    crew_render::DEFAULT_TEXT_GAMMA
}
