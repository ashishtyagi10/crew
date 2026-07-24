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
    ///
    /// Restricted to [`crew_theme::FONT_ALLOWLIST`]: crew only ever
    /// auto-selects (rotation + theme resolution both draw from this) a
    /// curated set of coding faces, so a rotation can never land on Courier or
    /// another typewriter face. If a machine has *none* of the allowlisted
    /// families installed, we fall back to the full installed set rather than
    /// leave the app with no font at all.
    pub(crate) fn font_pool(&mut self) -> Vec<String> {
        if self.font_rotate.pool.is_none() {
            let installed = self
                .renderer
                .as_mut()
                .map(|r| r.monospace_families())
                .unwrap_or_default();
            self.font_rotate.pool = Some(allowed_pool(installed));
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

    /// The first family in `prefs` that is actually installed.
    ///
    /// Themes state a preference, not a font: a family that isn't installed
    /// makes fontdb substitute a proportional face and cell rounding then
    /// mangles every glyph. `None` = none of them are here, and the caller
    /// must leave the font alone rather than guess.
    pub(crate) fn resolve_family(&mut self, prefs: &[&str]) -> Option<String> {
        let pool = self.font_pool();
        prefs
            .iter()
            .find(|want| pool.iter().any(|have| have == *want))
            .map(|s| s.to_string())
    }
}

/// Keep only families in [`crew_theme::FONT_ALLOWLIST`]. If a machine has none
/// of them installed, fall back to the full set so the app still has a font
/// rather than none at all.
fn allowed_pool(installed: Vec<String>) -> Vec<String> {
    let allowed: Vec<String> = installed
        .iter()
        .filter(|f| crew_theme::FONT_ALLOWLIST.contains(&f.as_str()))
        .cloned()
        .collect();
    if allowed.is_empty() {
        installed
    } else {
        allowed
    }
}

#[cfg(test)]
#[path = "fontcmd_tests.rs"]
mod tests;
