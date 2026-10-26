//! Derived views over `CrewConfig`: theme/accent resolution, line height,
//! and range clamping. Split from `config.rs` (child module).
use super::*;

impl CrewConfig {
    /// One text row in logical pixels — font size × `leading`, the same
    /// product `crew_render::cell_metrics` takes, so sizing and cells agree.
    pub fn line_height(&self) -> f32 {
        self.font_size * self.leading().ratio()
    }

    /// The configured theme, or `paper-dark` when unset/unknown.
    pub fn theme_id(&self) -> crew_theme::ThemeId {
        self.theme
            .as_deref()
            .and_then(crew_theme::ThemeId::from_name)
            .unwrap_or(crew_theme::ThemeId::PaperDark)
    }

    /// The configured theme selection — the ONE resolution both startup
    /// (`handler.rs`) and config adoption (`apply_config`) go through, so
    /// they can never disagree. A saved value that parses keeps exactly its
    /// meaning (an explicit pick is an intent — never hijacked). **No saved
    /// value resolves to `auto`**, every look in turn. A saved value that
    /// does NOT parse falls back to the fixed default palette, as it always
    /// has — a broken string is not an intent to rotate.
    pub fn theme_selection(&self) -> crew_theme::Selection {
        match self.theme.as_deref() {
            None => crew_theme::Selection::Mode(crew_theme::RandomMode::Auto),
            Some(s) => crew_theme::parse_selection(s).unwrap_or(self.theme_id().selection()),
        }
    }

    /// Read the OS preferences crew follows and publish them. Call it where
    /// the answers can actually change (startup, ThemeChanged, config
    /// adoption), never per frame.
    pub fn publish_appearance_sources(&self) {
        // Reduce-motion: `auto` motion is stale the moment this is not
        // refreshed alongside the appearance.
        crate::motion::set_os_reduce(crate::reducemotion::reduce_motion());
        // Same three probe points, same rule: read where it can change, cache
        // it, never ask per frame.
        crew_theme::contrast::set_high_contrast(
            self.high_contrast(crate::oscontrast::increase_contrast()),
        );
        crate::shapecues::set(self.shape_cues(crate::shapecues::os_asks()));
        crate::motion::set_level(self.motion_level());
    }

    /// A display label for the configured selection: the rotation mode name
    /// (`dark`/`light`/`crt`/`auto`) if it is one, the pinned palette name if
    /// a specific palette is saved, or `auto` when unset (the fresh-install
    /// default). Used by the settings picker, which offers
    /// only the consolidated modes.
    pub fn theme_label(&self) -> String {
        match self.theme_selection() {
            crew_theme::Selection::Mode(m) => m.as_str().to_string(),
            crew_theme::Selection::Fixed(id) => id.as_str().to_string(),
        }
    }

    /// The configured accent colour, or the active theme's default when unset/invalid.
    pub fn accent_rgb(&self) -> (u8, u8, u8) {
        self.accent
            .as_deref()
            .and_then(crate::palette::parse_hex)
            .unwrap_or_else(|| crew_theme::theme().accent_default)
    }

    /// The configured frosted-glass strength; an unknown name falls back to the
    /// default rather than silently rendering flat.
    pub fn glass_level(&self) -> crew_theme::GlassLevel {
        crew_theme::GlassLevel::parse(&self.glass).unwrap_or(crew_theme::GlassLevel::Medium)
    }

    /// Whether crew should draw for high contrast right now: the user's
    /// setting, or the OS's answer when it is `auto`. An unknown name follows
    /// the OS — a typo must not quietly overrule an accessibility request.
    pub(crate) fn high_contrast(&self, os: bool) -> bool {
        match self.contrast.trim().to_ascii_lowercase().as_str() {
            "high" | "on" | "more" => true,
            "normal" | "off" | "aa" => false,
            _ => os,
        }
    }

    /// Whether crew should add shape cues right now: the user's setting, or
    /// the OS's answer when it is `auto`. An unknown name follows the OS — a
    /// typo must not quietly overrule an accessibility request.
    pub(crate) fn shape_cues(&self, os: bool) -> bool {
        match self.shape_cues.trim().to_ascii_lowercase().as_str() {
            "on" | "shapes" | "always" => true,
            "off" | "never" => false,
            _ => os,
        }
    }

    /// The configured density; an unknown name falls back to `cozy`, the
    /// default — a typo must not re-space the whole canvas.
    pub(crate) fn density(&self) -> crate::density::Density {
        crate::density::Density::parse(&self.density).unwrap_or(crate::density::Density::Cozy)
    }

    /// The configured leading; an unknown name falls back to `normal`, the
    /// default — a typo must not silently re-space every line of every pane.
    pub(crate) fn leading(&self) -> crate::leading::Leading {
        crate::leading::Leading::parse(&self.leading).unwrap_or(crate::leading::Leading::Normal)
    }

    /// The configured motion preference; an unknown name falls back to `auto`,
    /// matching the default — a typo must not silently disable animation, nor
    /// silently overrule the OS.
    pub(crate) fn motion_pref(&self) -> crate::motion::MotionPref {
        crate::motion::MotionPref::parse(&self.motion).unwrap_or(crate::motion::MotionPref::Auto)
    }

    /// The motion strength that actually renders: the preference resolved
    /// against the last-published OS "reduce motion" answer.
    pub(crate) fn motion_level(&self) -> crate::motion::MotionLevel {
        self.motion_pref().resolve(crate::motion::os_reduce())
    }

    /// The configured gradient level; an unknown name falls back to `subtle`,
    /// the default, so a typo softens the effect rather than pinning the
    /// poles or over-driving them.
    pub(crate) fn gradient_level(&self) -> crate::gradientlvl::GradientLevel {
        crate::gradientlvl::GradientLevel::parse(&self.gradient)
            .unwrap_or(crate::gradientlvl::GradientLevel::Subtle)
    }
}
