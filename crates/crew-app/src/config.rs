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

fn default_font_weight() -> u16 {
    // SemiBold. Heavier than the old Medium (500) base so body text reads
    // thicker and more substantial out of the box; /weight tunes it live.
    600
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CrewConfig {
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
    /// Base text weight on the CSS scale (400 normal … 900 black). Defaults to
    /// SemiBold (600) for a thicker body; set live with `/weight`.
    #[serde(default = "default_font_weight")]
    pub font_weight: u16,
    /// Token budgets for the footer's rolling usage windows (the `%` the
    /// bars are drawn against). Approximate by nature — tune to taste.
    #[serde(default = "default_usage_budget_5h")]
    pub usage_budget_5h: u64,
    #[serde(default = "default_usage_budget_7d")]
    pub usage_budget_7d: u64,
}

impl Default for CrewConfig {
    fn default() -> Self {
        Self {
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
            paper_texture: true,
            paper_grain: default_paper_grain(),
            crt: None,
            font_weight: default_font_weight(),
            usage_budget_5h: default_usage_budget_5h(),
            usage_budget_7d: default_usage_budget_7d(),
        }
    }
}

impl CrewConfig {}

#[path = "configio.rs"]
mod configio;
#[path = "configread.rs"]
mod configread;

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
