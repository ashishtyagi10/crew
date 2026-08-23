use std::path::PathBuf;

fn default_font_size() -> f32 {
    14.0
}

fn default_nav_width() -> f32 {
    210.0
}

fn default_show_nav() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_notify_min_secs() -> u64 {
    10
}

fn default_usage_budget_5h() -> u64 {
    5_000_000
}
fn default_usage_budget_7d() -> u64 {
    25_000_000
}

fn default_paper_grain() -> f32 {
    // ~2.6% luminance grain — clearly reads as paper texture without looking
    // noisy (chosen by comparing a rendered 0.0/0.6/1.0/1.6 sweep). Tunable in
    // config; 0.0 disables grain, paper_texture=false disables the whole pass.
    1.3
}

fn default_motion() -> String {
    // Crew moves by default. `off` is the reduce-motion setting and costs
    // nothing extra to render (see `motion::MotionLevel`).
    "full".to_string()
}

fn default_auto_light_from() -> String {
    "07:00".to_string()
}

fn default_auto_light_to() -> String {
    "19:00".to_string()
}

fn default_glass() -> String {
    // Strength only; the look is derived per-theme (see `crew_theme::glass`).
    // Since 2026-08-06 paper themes derive a flat (invisible) sheet, so this
    // dial only shows on CRT themes — `off` kills even the holographic sheet.
    "medium".to_string()
}

/// Floor for [`CrewConfig::window_opacity`]. Mirrors the renderer's own clamp:
/// a window dialled to invisible is a window the user cannot get back.
pub const MIN_WINDOW_OPACITY: f32 = 0.35;

fn default_window_opacity() -> f32 {
    // Opaque. Window translucency is opt-in via Settings → WINDOW → Opacity % — a
    // see-through terminal is a taste, not a default.
    1.0
}

/// Whether the window should composite as non-opaque *right now*.
///
/// Not the same question as "can this window ever be translucent". That one is
/// answered once, at creation, by `.with_transparent(true)` in `handler` — it
/// cannot be changed later without tearing the window down, which is why crew
/// asks for it unconditionally.
///
/// This is the runtime flag, and on macOS it drives `NSWindow.isOpaque`. That
/// matters because the **title bar is drawn by the OS, not by crew**: a
/// non-opaque window composites its title bar against whatever is behind it.
/// Leaving the flag on at full opacity is what made the title bar show the
/// desktop through while every pane stayed solid — `handler`'s "nothing crew
/// draws leaves alpha below 1" is true and was never about the chrome.
pub fn wants_window_transparency(opacity: f32) -> bool {
    opacity < 1.0
}

fn default_font_weight() -> u16 {
    // SemiBold. Heavier than the old Medium (500) base so body text reads
    // thicker and more substantial out of the box; /weight tunes it live.
    600
}

fn default_font_smooth() -> u8 {
    // The renderer's calibrated CoreText-style stem darkening.
    crew_render::DEFAULT_SMOOTH
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CrewConfig {
    /// The version that last ran. Compared at startup so a build the user
    /// did not choose to install — auto-update lands quietly — can say what
    /// it changed rather than arriving as a silently different app.
    #[serde(default)]
    pub last_seen_version: Option<String>,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_nav_width")]
    pub nav_width: f32,
    #[serde(default = "default_show_nav")]
    pub show_nav: bool,
    /// Chosen font family; `None`/empty uses the system monospace.
    #[serde(default)]
    pub font_family: Option<String>,
    /// `/font random`: rotate the UI font every 10 minutes through the
    /// installed monospace families. The rotated pick itself is NOT saved —
    /// `font_family` stays whatever the user pinned.
    #[serde(default)]
    pub font_random: bool,
    /// Accent colour as a `#rrggbb` hex string; `None`/invalid uses the built-in
    /// Crew green. Applied app-wide via [`crate::palette`].
    #[serde(default)]
    pub accent: Option<String>,
    /// Whether the window should launch maximized.
    #[serde(default)]
    pub maximized: bool,
    /// Last working directory (absolute), restored on the next launch.
    #[serde(default)]
    pub last_dir: Option<String>,
    /// Last window size in logical pixels, restored on the next launch.
    #[serde(default)]
    pub win_w: Option<f32>,
    #[serde(default)]
    pub win_h: Option<f32>,
    /// Master switch for the notification system (pane events flashed on the
    /// input bar + logged in the sidebar). When off, no events are surfaced.
    #[serde(default = "default_true")]
    pub notify: bool,
    /// Notify when a foreground command in a pane finishes (returns to the shell
    /// prompt) after running at least `notify_min_secs`.
    #[serde(default = "default_true")]
    pub notify_agent_done: bool,
    /// Notify when a program rings the terminal bell.
    #[serde(default = "default_true")]
    pub notify_bell: bool,
    /// Notify when a pane's process exits.
    #[serde(default = "default_true")]
    pub notify_exit: bool,
    /// Minimum foreground-command runtime (seconds) before a "finished"
    /// notification fires — suppresses quick commands like `ls`/`cd`.
    #[serde(default = "default_notify_min_secs")]
    pub notify_min_secs: u64,
    /// Case-insensitive substrings watched in pane output; a match notifies.
    #[serde(default)]
    pub notify_patterns: Vec<String>,
    /// Theme name: `paper-dark` (default) or `paper-light`. Unknown/unset →
    /// `paper-dark`. Applied app-wide via [`crew_theme`].
    #[serde(default)]
    pub theme: Option<String>,
    /// While `theme = "auto"`: what serves when the OS is in dark mode — a
    /// rotation pool (`dark` | `light` | `crt`) or a pinned palette name
    /// (e.g. `crt-green`). Unset → the dark paper pool. `auto` itself is
    /// rejected (it can't serve as its own side).
    #[serde(default)]
    pub theme_dark: Option<String>,
    /// While `theme = "auto"`: what serves when the OS is in light mode.
    /// Same values as `theme_dark`; unset → the light paper pool.
    #[serde(default)]
    pub theme_light: Option<String>,
    /// While `theme = "auto"` and the OS appearance is PINNED (not on macOS
    /// Appearance: Auto): the local `HH:MM` daylight starts and ends. crew
    /// has no location, so this is a wall-clock window rather than real
    /// sunrise/sunset — dial it to your own. An unparseable value falls back
    /// to the 07:00–19:00 default; a window whose end is at or before its
    /// start wraps past midnight (see `daylight`).
    #[serde(default = "default_auto_light_from")]
    pub auto_light_from: String,
    /// End of the light-hours window; see `auto_light_from`.
    #[serde(default = "default_auto_light_to")]
    pub auto_light_to: String,
    /// Whether to render the subtle paper grain + vignette background texture.
    /// When off, the window background is a plain flat colour.
    #[serde(default = "default_true")]
    pub paper_texture: bool,
    /// Grain amplitude multiplier for the paper texture (0.0 = no grain, 1.0 = default ~3%, 2.0 = double).
    #[serde(default = "default_paper_grain")]
    pub paper_grain: f32,
    /// CRT tube post-process override. `None` (default) follows the active
    /// theme's `crt` flag — on for the `crt-*` phosphor themes, off elsewhere.
    /// `Some(true)`/`Some(false)` forces it via `/crt on|off` regardless of
    /// theme.
    #[serde(default)]
    pub crt: Option<bool>,
    /// Frosted-glass strength for pane cards: `off`, `low`, `medium`, `high`.
    /// The per-theme look is derived from the active theme, so this is the
    /// intensity knob only. Set in Settings → APPEARANCE → Glass.
    #[serde(default = "default_glass")]
    pub glass: String,

    /// How much crew animates: `off`, `subtle`, `full`. An unknown name falls
    /// back to `full`. Set in Settings → APPEARANCE → Motion.
    #[serde(default = "default_motion")]
    pub motion: String,
    /// Window opacity, `1.0` = fully opaque. Below 1.0 the desktop shows
    /// through the page (text and pane fills stay solid). Settings → WINDOW.
    #[serde(default = "default_window_opacity")]
    pub window_opacity: f32,
    /// Base text weight on the CSS scale (400 normal … 900 black). Defaults to
    /// SemiBold (600) for a thicker body; set live with `/weight`.
    #[serde(default = "default_font_weight")]
    pub font_weight: u16,
    /// CoreText-style font smoothing strength (0–255, 0 = off). Emulates the
    /// stem darkening every native macOS terminal gets from CoreText, so the
    /// same font reads as full here as in Terminal.app; `/smooth` tunes it.
    #[serde(default = "default_font_smooth")]
    pub font_smooth: u8,
    /// Token budgets for the footer's rolling usage windows (the `%` the
    /// bars are drawn against). Approximate by nature — tune to taste.
    #[serde(default = "default_usage_budget_5h")]
    pub usage_budget_5h: u64,
    #[serde(default = "default_usage_budget_7d")]
    pub usage_budget_7d: u64,
    /// Recently picked models, most recent first (cap 5) — the `/model`
    /// picker's shortcut section. Slugs only; unknown ones are skipped.
    #[serde(default)]
    pub model_recents: Vec<String>,
}

impl Default for CrewConfig {
    fn default() -> Self {
        Self {
            last_seen_version: None,
            font_size: default_font_size(),
            nav_width: default_nav_width(),
            show_nav: default_show_nav(),
            font_family: None,
            font_random: false,
            accent: None,
            maximized: false,
            last_dir: None,
            win_w: None,
            win_h: None,
            notify: true,
            notify_agent_done: true,
            notify_bell: true,
            notify_exit: true,
            notify_min_secs: default_notify_min_secs(),
            notify_patterns: Vec::new(),
            theme: None,
            theme_dark: None,
            theme_light: None,
            auto_light_from: default_auto_light_from(),
            auto_light_to: default_auto_light_to(),
            paper_texture: true,
            paper_grain: default_paper_grain(),
            crt: None,
            glass: default_glass(),
            motion: default_motion(),
            window_opacity: default_window_opacity(),
            font_weight: default_font_weight(),
            font_smooth: default_font_smooth(),
            usage_budget_5h: default_usage_budget_5h(),
            usage_budget_7d: default_usage_budget_7d(),
            model_recents: Vec::new(),
        }
    }
}

impl CrewConfig {
    /// Clear the look-killing overrides so the newly chosen theme shows as
    /// designed: a `/crt on|off` pin returns to auto (follow the theme), and a
    /// glass strength of `off` returns to the frosted default. A deliberate
    /// `low`/`high` glass strength is taste, not a kill switch, and survives.
    /// Returns true when anything changed. Without this, a pin from months ago
    /// silently wins over every later theme switch — the "CRT theme is just a
    /// dark theme" failure.
    pub fn reset_look_overrides(&mut self) -> bool {
        let mut changed = false;
        if self.crt.is_some() {
            self.crt = None;
            changed = true;
        }
        if matches!(
            crew_theme::GlassLevel::parse(&self.glass),
            Some(crew_theme::GlassLevel::Off)
        ) {
            self.glass = default_glass();
            changed = true;
        }
        changed
    }
}

#[path = "configio.rs"]
mod configio;
#[path = "configread.rs"]
mod configread;

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
