//! `/font [size|random]`: set the font size to an exact value, or turn on a
//! 10-minute rotation over the installed monospace families. The `Cmd+=` /
//! `Cmd+-` chords only step the size by one; this jumps straight to a size
//! (handy for screenshots or presentations). With no argument it reports the
//! current size and rotation state.
use crate::app::CrewApp;

impl CrewApp {
    /// Set the font size from `arg` (a number), toggle rotation with
    /// `arg == "random"`, or report the current size + rotation state when
    /// `arg` is empty. Out-of-range sizes are clamped (12–32) by `set_font`.
    pub(crate) fn set_font_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.eq_ignore_ascii_case("random") {
            if self.font_rotate.on {
                // Toggle off: back to the pinned family (or system monospace).
                self.stop_font_rotation();
                return;
            }
            let pool = self.font_pool();
            let now = crate::chattime::unix_now_ms();
            let seed = now;
            let cur = self.current_family();
            match crate::fontrotate::pick(&pool, cur.as_deref(), seed) {
                Some(fam) => {
                    self.font_rotate.on = true;
                    self.font_rotate.last_ms = now;
                    self.apply_rotated_family(fam);
                    self.config.font_random = true;
                    self.config.save();
                }
                None => {
                    self.font_rotate.on = false;
                    // Clear a stale saved flag too (a one-font machine would
                    // otherwise resume useless no-op rotation every launch).
                    if self.config.font_random {
                        self.config.font_random = false;
                        self.config.save();
                    }
                    self.set_status("font random: only one monospace font installed".to_string());
                }
            }
            return;
        }
        if arg.is_empty() {
            let rot = if self.font_rotate.on {
                // Before the first rotated pick lands, report the family the
                // rotation is starting from (the pinned one).
                match self.current_family() {
                    Some(f) => format!(" — rotating (now: {f})"),
                    None => " — rotating".to_string(),
                }
            } else {
                String::new()
            };
            self.set_status(format!(
                "font size {}{rot} — /font <n> to set, /font random to toggle rotation",
                self.config.font_size as i32
            ));
            return;
        }
        match arg.parse::<f32>() {
            Ok(n) => self.set_font(n),
            Err(_) => self.set_status(format!("font: not a number: {arg}")),
        }
    }

    /// The cached monospace pool, scanning once on first use (loads faces).
    pub(crate) fn font_pool(&mut self) -> Vec<String> {
        if self.font_rotate.pool.is_none() {
            let pool = self
                .renderer
                .as_mut()
                .map(|r| r.monospace_families())
                .unwrap_or_default();
            self.font_rotate.pool = Some(pool);
        }
        self.font_rotate.pool.clone().unwrap_or_default()
    }

    /// The family rotation should avoid repeating: the rotated pick if one is
    /// live, else the pinned config family.
    pub(crate) fn current_family(&self) -> Option<String> {
        self.font_rotate
            .current
            .clone()
            .or_else(|| self.config.font_family.clone())
    }

    /// Rotate the font if a rotation is due at `now_ms`; returns whether a new
    /// family was applied.
    ///
    /// Split out of `poll_panes` so the wiring is testable at all: that loop
    /// returns early without a window, so the *enabled* rotation path had no
    /// coverage — `due`/`pick` were only ever tested in isolation, and the one
    /// headless test asserts rotation stays off (no renderer → empty pool).
    /// A feature can be wholly dead with the suite green.
    pub(crate) fn tick_font_rotation(&mut self, now_ms: u64) -> bool {
        if !self.font_rotate.due(now_ms) {
            return false;
        }
        let pool = self.font_pool();
        let cur = self.current_family();
        let picked = crate::fontrotate::pick(&pool, cur.as_deref(), now_ms);
        // Stamp the clock even when the pool offered no alternative, so a
        // one-font machine retries in 10 minutes rather than on every tick.
        self.font_rotate.last_ms = now_ms;
        match picked {
            Some(fam) => {
                self.apply_rotated_family(fam);
                true
            }
            None => false,
        }
    }

    /// Apply a rotated family to the renderer and status line — NEVER to config.
    pub(crate) fn apply_rotated_family(&mut self, fam: String) {
        if let Some(r) = &mut self.renderer {
            r.set_font_family(Some(fam.clone()));
        }
        self.set_status(format!("font → {fam}"));
        self.font_rotate.current = Some(fam);
        self.redraw();
    }

    /// Turn rotation off and restore the pinned config family (the `/font
    /// random` toggle-off path; the Settings manual-pick override clears the
    /// same state inline in `apply_config`, where the pinned family is being
    /// re-applied anyway).
    pub(crate) fn stop_font_rotation(&mut self) {
        self.font_rotate.on = false;
        self.font_rotate.current = None;
        if let Some(r) = &mut self.renderer {
            r.set_font_family(self.config.font_family.clone());
        }
        self.config.font_random = false;
        self.config.save();
        let back = self
            .config
            .font_family
            .clone()
            .unwrap_or_else(|| "system monospace".to_string());
        self.set_status(format!("font rotation off — back to {back}"));
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn parses_and_clamps_to_range() {
        // A font-SIZE test needs the theme guard: `set_font` persists through
        // `apply_settings` → `apply_config`, which pins a fixed theme app-wide
        // and clears the random MODE. Without this it races
        // `chataction::persist_theme_saves_the_live_mode_name`, which then
        // reads "paper-dark" instead of its own "random-light".
        let _g = crate::app::theme_test_guard();
        let mut app = CrewApp::default();
        app.set_font_cmd("18");
        assert_eq!(app.config.font_size, 18.0);
        app.set_font_cmd("5"); // below min → clamps up
        assert_eq!(app.config.font_size, 12.0);
        app.set_font_cmd("999"); // above max → clamps down
        assert_eq!(app.config.font_size, 32.0);
    }

    #[test]
    fn rejects_non_number_without_changing_size() {
        let mut app = CrewApp::default();
        let before = app.config.font_size;
        app.set_font_cmd("big");
        assert_eq!(app.config.font_size, before);
        assert!(app.active_status().is_some());
    }

    #[test]
    fn font_random_arg_enables_rotation_or_reports_thin_pool() {
        let mut app = CrewApp::default();
        app.set_font_cmd("random");
        // Headless default app has no renderer → pool scan yields nothing →
        // rotation must stay off with the thin-pool report.
        assert!(!app.font_rotate.on);
        assert!(app.active_status().is_some());
    }

    /// Seed the pool the way a live renderer scan would. Headless there is no
    /// renderer, so `font_pool` caches an EMPTY vec and `pick` returns None —
    /// which is exactly why the enabled path had never been exercised.
    fn rotating_app() -> CrewApp {
        let mut app = CrewApp::default();
        app.font_rotate.pool = Some(vec!["Menlo".into(), "Monaco".into()]);
        app.font_rotate.on = true;
        app.font_rotate.current = Some("Menlo".into());
        app.font_rotate.last_ms = 0;
        app
    }

    #[test]
    fn a_due_rotation_applies_a_new_family() {
        // The wiring test that did not exist: `due` and `pick` were covered in
        // isolation, so nothing proved a due rotation reaches the renderer.
        let mut app = rotating_app();
        let now = crew_theme::ROTATE_MS;
        assert!(app.tick_font_rotation(now), "a due rotation must apply");
        assert_eq!(app.font_rotate.current.as_deref(), Some("Monaco"));
        assert_eq!(app.font_rotate.last_ms, now, "clock must restamp");
    }

    #[test]
    fn rotation_does_not_fire_before_the_clock_elapses() {
        let mut app = rotating_app();
        assert!(!app.tick_font_rotation(crew_theme::ROTATE_MS - 1));
        assert_eq!(
            app.font_rotate.current.as_deref(),
            Some("Menlo"),
            "family must not change early"
        );
    }

    #[test]
    fn rotation_does_not_fire_while_off() {
        let mut app = rotating_app();
        app.font_rotate.on = false;
        assert!(!app.tick_font_rotation(crew_theme::ROTATE_MS));
        assert_eq!(app.font_rotate.current.as_deref(), Some("Menlo"));
    }

    #[test]
    fn rotation_keeps_firing_on_each_subsequent_clock() {
        // One rotation working is not the reported symptom — "sets a font once
        // and stops" is. Prove the SECOND rotation lands too.
        let mut app = rotating_app();
        assert!(app.tick_font_rotation(crew_theme::ROTATE_MS));
        assert_eq!(app.font_rotate.current.as_deref(), Some("Monaco"));
        assert!(
            app.tick_font_rotation(crew_theme::ROTATE_MS * 2),
            "the second rotation must fire too"
        );
        assert_eq!(
            app.font_rotate.current.as_deref(),
            Some("Menlo"),
            "two-font pool must swing back, not stick"
        );
    }

    #[test]
    fn no_arg_report_mentions_rotation_state() {
        let mut app = CrewApp::default();
        app.set_font_cmd("");
        let s = app.active_status().unwrap();
        assert!(s.contains("font size"), "{s}");
    }

    #[test]
    fn font_random_while_rotating_toggles_off_and_restores_pinned() {
        let mut app = CrewApp::default();
        app.config.font_family = Some("Pinned Mono".to_string());
        app.font_rotate.on = true;
        app.font_rotate.current = Some("Rotated Mono".to_string());
        app.config.font_random = true;
        app.set_font_cmd("random");
        assert!(
            !app.font_rotate.on,
            "second /font random turns rotation off"
        );
        assert!(app.font_rotate.current.is_none());
        assert!(!app.config.font_random);
        assert_eq!(app.config.font_family.as_deref(), Some("Pinned Mono"));
        let s = app.active_status().unwrap();
        assert!(s.contains("rotation off"), "{s}");
    }

    #[test]
    fn rotation_never_touches_the_pinned_config_family() {
        // The feature's core safety property: a rotated pick lives on
        // font_rotate.current ONLY, so unrelated config.save() calls (the
        // resize settle, /theme) can never persist it.
        let mut app = CrewApp::default();
        app.config.font_family = Some("Pinned Mono".to_string());
        app.apply_rotated_family("Rotated Mono".to_string());
        assert_eq!(app.config.font_family.as_deref(), Some("Pinned Mono"));
        assert_eq!(app.font_rotate.current.as_deref(), Some("Rotated Mono"));
    }
}
