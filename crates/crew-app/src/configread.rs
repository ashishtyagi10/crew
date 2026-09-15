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
    /// value resolves to the OS-following `auto` mode**: a fresh install
    /// comes up light on a light system, dark on a dark one. A saved value
    /// that does NOT parse falls back to the fixed default palette, as it
    /// always has — a broken string is not an intent to follow the OS.
    pub fn theme_selection(&self) -> crew_theme::Selection {
        match self.theme.as_deref() {
            None => crew_theme::Selection::Mode(crew_theme::RandomMode::Auto),
            Some(s) => crew_theme::parse_selection(s)
                .unwrap_or(crew_theme::Selection::Fixed(self.theme_id())),
        }
    }

    /// The `auto` theme's per-appearance pairing (`theme_dark` /
    /// `theme_light`), parsed: each side is a pool mode or pinned palette.
    /// `auto` itself is rejected as a side (it can't serve itself — crew-theme
    /// would drop it anyway; rejecting here keeps config's view honest), and
    /// unknown names fall back to `None` = that side's built-in paper pool.
    pub fn auto_pool_selections(
        &self,
    ) -> (Option<crew_theme::Selection>, Option<crew_theme::Selection>) {
        let parse = |s: &Option<String>| {
            s.as_deref()
                .and_then(crew_theme::parse_selection)
                .filter(|sel| *sel != crew_theme::Selection::Mode(crew_theme::RandomMode::Auto))
        };
        (parse(&self.theme_dark), parse(&self.theme_light))
    }

    /// `auto`'s light-hours window as minutes past midnight, used only when
    /// the OS appearance is pinned. Each end falls back to its default
    /// INDEPENDENTLY: a typo in one bound shouldn't silently redefine the
    /// other, and a half-parsed window is still a window the user can read
    /// back off `/theme`.
    pub fn light_hours(&self) -> (u16, u16) {
        (
            crate::daylight::parse_hhmm(&self.auto_light_from)
                .unwrap_or(crate::daylight::DEFAULT_FROM),
            crate::daylight::parse_hhmm(&self.auto_light_to).unwrap_or(crate::daylight::DEFAULT_TO),
        )
    }

    /// Push the local clock's day/night verdict into `crew_theme`, returning
    /// it. Clock only, so it is cheap enough for the poll tick — the pinned/
    /// scheduled probe it pairs with is [`Self::publish_os_auto`], which reads
    /// OS preferences and belongs on the rare paths instead.
    pub fn publish_daylight(&self) -> bool {
        let (from, to) = self.light_hours();
        // Republished with the verdict so `/theme` can never quote a window
        // that isn't the one the verdict came from.
        crew_theme::set_light_hours(from, to);
        let day = crate::daylight::is_day_now(from, to);
        crew_theme::set_daylight(day);
        day
    }

    /// Probe whether the OS switches its own appearance and publish that too.
    /// Reads OS preferences: call it where the answer can actually change
    /// (startup, ThemeChanged, config adoption), never per frame.
    pub fn publish_appearance_sources(&self) -> bool {
        crew_theme::set_os_auto(crate::osappearance::switches_automatically());
        // Reduce-motion rides the same probe points: it is an OS preference
        // read through the same "only where it can change" rule, and `auto`
        // motion is stale the moment this is not refreshed alongside.
        crate::motion::set_os_reduce(crate::reducemotion::reduce_motion());
        // Same three probe points, same rule: read where it can change, cache
        // it, never ask per frame.
        crew_theme::contrast::set_high_contrast(
            self.high_contrast(crate::oscontrast::increase_contrast()),
        );
        crate::shapecues::set(self.shape_cues(crate::shapecues::os_asks()));
        crate::motion::set_level(self.motion_level());
        self.publish_daylight()
    }

    /// A display label for the configured selection: the rotation mode name
    /// (`dark`/`light`/`crt`/`auto`) if it is one, the pinned palette name if
    /// a specific palette is saved, or `auto` when unset (the fresh-install
    /// default follows the OS). Used by the settings picker, which offers
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
