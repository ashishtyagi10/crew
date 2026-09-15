//! `CrewConfig::clamped`: every value read off disk pulled back into range.
//!
//! Its own file because of what it is — a rebuild of the WHOLE struct, field
//! by field. A field left out of it is not a field that keeps its saved value:
//! it is a field reset to the literal written here, on every single load, for
//! as long as nobody notices (`last_seen_version` was dead from birth that
//! way, which is why there were no version notes and no migrations). So it is
//! kept where the eye can hold all of it at once.
use super::*;

impl CrewConfig {
    /// Every value read off disk pulled back into range.
    pub fn clamped(self) -> Self {
        Self {
            // MUST carry through: `load()` clamps, so dropping this made every
            // launch a first run (no version note, no config migrations).
            last_seen_version: self.last_seen_version,
            font_size: self.font_size.clamp(12.0, 32.0),
            nav_width: self.nav_width.clamp(160.0, 320.0),
            show_nav: self.show_nav,
            nav_collapsed: self.nav_collapsed,
            border_marks: self.border_marks,
            invisibles: self.invisibles,
            lsp: self.lsp,
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
            nav_card: self.nav_card,
            weather_place: self.weather_place,
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
