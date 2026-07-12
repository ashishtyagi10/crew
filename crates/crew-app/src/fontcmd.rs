//! `/font [size|random]`: set the font size to an exact value, or turn on a
//! 10-minute rotation over the installed monospace families. The `Cmd+=` /
//! `Cmd+-` chords only step the size by one; this jumps straight to a size
//! (handy for screenshots or presentations). With no argument it reports the
//! current size and rotation state.
use crate::app::CrewApp;

impl CrewApp {
    /// Set the font size from `arg` (a number), turn on rotation with
    /// `arg == "random"`, or report the current size + rotation state when
    /// `arg` is empty. Out-of-range sizes are clamped (12–32) by `set_font`.
    pub(crate) fn set_font_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.eq_ignore_ascii_case("random") {
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
                    self.set_status("font random: only one monospace font installed".to_string());
                }
            }
            return;
        }
        if arg.is_empty() {
            let rot = if self.font_rotate.on {
                match &self.font_rotate.current {
                    Some(f) => format!(" — rotating (now: {f})"),
                    None => " — rotating".to_string(),
                }
            } else {
                String::new()
            };
            self.set_status(format!(
                "font size {}{rot} — /font <n> to set, /font random to rotate",
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

    /// Apply a rotated family to the renderer and status line — NEVER to config.
    pub(crate) fn apply_rotated_family(&mut self, fam: String) {
        if let Some(r) = &mut self.renderer {
            r.set_font_family(Some(fam.clone()));
        }
        self.set_status(format!("font → {fam}"));
        self.font_rotate.current = Some(fam);
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn parses_and_clamps_to_range() {
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

    #[test]
    fn no_arg_report_mentions_rotation_state() {
        let mut app = CrewApp::default();
        app.set_font_cmd("");
        let s = app.active_status().unwrap();
        assert!(s.contains("font size"), "{s}");
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
