//! Derived views over `CrewConfig`: theme/accent resolution, line height,
//! and range clamping. Split from `config.rs` (child module).
use super::*;

impl CrewConfig {
    /// The height of one text row in logical pixels — the font size times the
    /// user's `leading`. The same product `crew_render::cell_metrics` takes,
    /// so the window-sizing math and the cell box can never disagree about
    /// how tall a row is.
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
    /// default — a typo must not silently re-space the whole canvas.
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

    pub fn clamped(self) -> Self {
        Self {
            // MUST carry through: `load()` clamps, so dropping this here made
            // every launch look like a first run — the "updated to crew X"
            // note never fired and version-gated config migrations never ran.
            last_seen_version: self.last_seen_version,
            font_size: self.font_size.clamp(12.0, 32.0),
            nav_width: self.nav_width.clamp(160.0, 320.0),
            show_nav: self.show_nav,
            border_marks: self.border_marks,
            invisibles: self.invisibles,
            font_family: self.font_family.filter(|n| !n.is_empty()),
            font_random: self.font_random,
            accent: self.accent.filter(|s| !s.is_empty()),
            maximized: self.maximized,
            last_dir: self.last_dir,
            win_w: self.win_w.map(|w| w.clamp(400.0, 10000.0)),
            win_h: self.win_h.map(|h| h.clamp(300.0, 10000.0)),
            notify: self.notify,
            notify_agent_done: self.notify_agent_done,
            notify_bell: self.notify_bell,
            notify_exit: self.notify_exit,
            notify_min_secs: self.notify_min_secs.clamp(1, 3600),
            notify_patterns: self
                .notify_patterns
                .into_iter()
                .filter(|p| !p.is_empty())
                .collect(),
            theme: self.theme.filter(|s| !s.is_empty()),
            theme_dark: self.theme_dark.filter(|s| !s.is_empty()),
            theme_light: self.theme_light.filter(|s| !s.is_empty()),
            auto_light_from: self.auto_light_from,
            auto_light_to: self.auto_light_to,
            paper_texture: self.paper_texture,
            ambient_drift: self.ambient_drift,
            paper_grain: self.paper_grain.clamp(0.0, 2.0),
            crt: self.crt,
            glass: self.glass,
            motion: self.motion,
            density: self.density,
            leading: self.leading,
            contrast: self.contrast,
            shape_cues: self.shape_cues,
            // Capped on the way in as well as on the way out: a hand-edited
            // or older config must not smuggle a longer history past the cap.
            command_recents: {
                let mut v = self.command_recents;
                v.truncate(crate::cmdrecents::MAX);
                v
            },
            gradient: self.gradient,
            gradient_poles: self.gradient_poles.filter(|s| !s.is_empty()),
            // A window that can be dialled to invisible is a window you cannot
            // find again; the floor keeps crew recoverable from any setting.
            window_opacity: self.window_opacity.clamp(MIN_WINDOW_OPACITY, 1.0),
            font_weight: self.font_weight.clamp(300, 900),
            // Any u8 is a valid smoothing strength; 0 simply turns it off.
            font_smooth: self.font_smooth,
            // Same for the gamma correction: the whole 0–255 range is legal.
            font_gamma: self.font_gamma,
            usage_budget_5h: self.usage_budget_5h.max(10_000),
            usage_budget_7d: self.usage_budget_7d.max(10_000),
            model_recents: self.model_recents,
        }
    }
}
