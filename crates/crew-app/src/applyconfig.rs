//! Putting a changed configuration into effect: the whole-config apply, the
//! font, and the theme command.
//!
//! Split out of [`crate::spawn`] for the line cap. Spawning a pane and
//! adopting a setting are different jobs that happened to share a file.
use crate::app::CrewApp;
use crate::config::CrewConfig;

impl CrewApp {
    /// Apply updated config: set font family + size live, persist to disk, and redraw.
    pub(crate) fn apply_settings(&mut self, cfg: CrewConfig) {
        self.apply_config(cfg);
        self.config.save();
    }

    /// Adopt `cfg` and apply it live (font family/size to the renderer, and a
    /// redraw to pick up nav width/visibility) *without* writing it back — used
    /// by `apply_settings`, which then persists.
    pub(crate) fn apply_config(&mut self, cfg: CrewConfig) {
        let old_family = self.config.font_family.clone();
        let old_pools = self.config.auto_pool_selections();
        self.config = cfg;
        // Apply theme selection: if the saved theme is a rotation mode name,
        // resume rotation in its pool (dark, light, or OS-following); if it's a
        // fixed theme name, pin that theme and stop rotation. This ensures a
        // theme chosen in the Settings pane isn't overridden by the rotation.
        // Reconcile ONLY when the config's selection differs from what's live.
        // `apply_selection(Mode(..))` re-picks a theme and restarts the
        // 10-minute clock, and every Settings save, `/theme`, and Cmd+= zoom
        // routes through `apply_settings` → here — so applying it
        // unconditionally re-rolled the theme on config touches that had
        // nothing to do with themes, and the rotation's own clock could never
        // run out. (It also made rotation look livelier than it is, masking
        // that the font rotation beside it has no such path.)
        // Auto's per-appearance pairing rides config too: republish it, and
        // if it changed while auto is the live mode, force a re-apply below —
        // "live" would otherwise be true (still Mode(Auto)) and a config edit
        // pairing night with the CRT pool wouldn't show until the next OS
        // flip or rotation tick.
        // The border markings ride config like the other look switches.
        crate::bordermarks::set(self.config.border_marks);
        crate::invisibles::set(self.config.invisibles);
        let (pool_dark, pool_light) = self.config.auto_pool_selections();
        let pools_changed = (pool_dark, pool_light) != old_pools;
        crew_theme::set_auto_pools(pool_dark, pool_light);
        // The light-hours window (`auto_light_from`/`auto_light_to`) rides
        // config the same way, so republish the clock sources here too and
        // treat a flipped verdict exactly like a changed pairing — otherwise
        // widening the window to cover right now wouldn't show until the next
        // tick crossed a boundary that no longer exists.
        let was_auto_dark = crew_theme::auto_dark();
        self.config.publish_appearance_sources();
        let auto_side_changed = pools_changed || crew_theme::auto_dark() != was_auto_dark;
        let want = self.config.theme_selection();
        let live = match want {
            crew_theme::Selection::Mode(m) => {
                crew_theme::mode() == Some(m)
                    && !(m == crew_theme::RandomMode::Auto && auto_side_changed)
            }
            crew_theme::Selection::Fixed(id) => {
                crew_theme::mode().is_none() && crew_theme::current_id() == id
            }
        };
        if !live {
            crew_theme::apply_selection(want, crate::chattime::unix_now_ms());
        }
        // Apply the themeable accent app-wide (render code reads it via palette).
        crate::palette::set_accent(self.config.accent_rgb());
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        if let Some(r) = &mut self.renderer {
            r.set_font_family(self.config.font_family.clone());
            r.set_font_size(self.config.font_size * scale);
            r.set_leading(self.config.leading().ratio());
            r.set_font_weight(Some(self.config.font_weight));
            r.set_text_smoothing(Some(self.config.font_smooth));
            r.set_text_gamma(Some(self.config.font_gamma));
            r.set_paper_texture(self.config.paper_texture);
            r.set_paper_grain(self.config.paper_grain);
        }
        // Glass rides the same path: a save that didn't push these two would
        // leave the sheet and the window opacity a restart behind. `/opacity`
        // sets the same value from the input bar and calls `apply_glass` for
        // exactly the same reason.
        self.apply_glass();
        crate::motion::set_level(self.config.motion_level());
        crate::density::set_level(self.config.density());
        // The gradient level rides the same path. Turning it OFF must also
        // put the poles back where they were — the shift is a live global,
        // and left where the last breath stopped it the canvas would keep a
        // colour the setting says it no longer wears.
        self.apply_gradient();
        // A manual family pick in Settings stops rotation; otherwise a live
        // rotation keeps its current pick on top of the re-applied config.
        if self.config.font_family != old_family {
            // Say so. This used to flip the flag silently, and the natural
            // reaction to a rotated pick you dislike — pinning your own font
            // back — is exactly what lands here, so rotation died without a
            // word and looked like "/font random only works once".
            let was_rotating = self.font_rotate.on;
            self.font_rotate.on = false;
            self.font_rotate.current = None;
            self.config.font_random = false;
            if was_rotating {
                let fam = self
                    .config
                    .font_family
                    .clone()
                    .unwrap_or_else(|| "system monospace".to_string());
                self.set_status(format!(
                    "font pinned: {fam} — rotation off (/font random to resume)"
                ));
            }
        } else if let (true, Some(fam)) = (self.font_rotate.on, self.font_rotate.current.clone()) {
            if let Some(r) = &mut self.renderer {
                r.set_font_family(Some(fam));
            }
        }
        // Pick up any change to the watched notification patterns on live panes.
        self.apply_notify_patterns();
        self.redraw();
    }

    /// Set the font size (clamped to the config's valid range), applying it live
    /// and persisting — shared by the Cmd+= / Cmd+- / Cmd+0 zoom chords.
    pub(crate) fn set_font(&mut self, size: f32) {
        let mut cfg = self.config.clone();
        cfg.font_size = size;
        self.apply_settings(cfg.clamped());
        self.set_status(format!("font size {}", self.config.font_size as i32));
    }

    /// `/theme [dark|light|crt|auto]`: switch the active theme live, persist
    /// the choice, and repaint. Each name enters a rotation over its palette
    /// pool (`auto`'s pool follows the OS appearance). Legacy names
    /// (`random-*` and the individual palette names) still resolve for
    /// back-compat. With no/unknown arg, report the current selection.
    pub(crate) fn set_theme_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status(crate::themereport::live_report());
            return;
        }
        let Some(sel) = crew_theme::parse_selection(arg) else {
            let names = crew_theme::THEME_MODES
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            // ERROR level, so it also steps onto the canvas as a toast: a
            // name this build doesn't know changes nothing on screen, and as
            // a three-second flash on the input bar's border the reason was
            // routinely missed — "/theme modern-light" on a build predating
            // that theme looked precisely like a theme that does nothing.
            self.set_status_err(format!("unknown theme '{arg}' ({names})"));
            return;
        };
        crew_theme::apply_selection(sel, crate::chattime::unix_now_ms());
        self.config.theme = Some(sel.label().to_string());
        // Re-apply the accent default (it follows the theme when the user hasn't
        // set an explicit accent).
        crate::palette::set_accent(self.config.accent_rgb());
        // Choosing a theme is a statement of intent: stale `/crt` pins and a
        // glass `off` from some earlier experiment stop overriding it.
        if self.config.reset_look_overrides() {
            self.apply_glass();
        }
        self.config.save();
        self.redraw();
        // Switching TO auto reports which half it just handed you, not the
        // bare word "auto" — same reason as the no-arg branch above.
        self.set_status(crate::themereport::live_report());
    }
}
