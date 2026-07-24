//! The per-tick font lifecycle: the 10-minute `/font random` rotation and
//! the theme-carried font, both driven from `poll_panes`. Split from
//! `fontcmd.rs` (the `/font` command + pool helpers) along the
//! command-vs-clock boundary; tests for both live in `fontcmd_tests.rs`.
use crate::app::CrewApp;

impl CrewApp {
    /// Apply the live theme's font when the theme has changed since last tick.
    /// Returns whether a family was applied.
    ///
    /// Ordered AFTER `tick_font_rotation` in `poll_panes` so that when both
    /// fire on the same tick — which they will, since both hang off the same
    /// 10-minute clock — the theme's font lands on top. Every other tick each
    /// simply wins by being the most recent event.
    pub(crate) fn tick_theme_font(&mut self) -> bool {
        let id = crew_theme::current_id();
        if self.font_rotate.themed == Some(id) {
            return false;
        }
        // Stamp first: an unresolvable preference must not retry every tick at
        // ~62 Hz, and a theme with no installed pick keeps the current font.
        self.font_rotate.themed = Some(id);
        let Some(fam) = self.resolve_family(crew_theme::font_prefs(id)) else {
            return false;
        };
        if self.current_family().as_deref() == Some(fam.as_str()) {
            return false; // already showing it — don't churn the atlas
        }
        self.apply_rotated_family(fam);
        true
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
