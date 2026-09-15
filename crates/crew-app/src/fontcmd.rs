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
            } else if let Some(fam) = &self.config.font_family {
                // A pin outranks the theme's own font (`tick_theme_font`), so
                // say which face is holding: otherwise the only sign that
                // themes stopped changing the typeface is that they stopped.
                format!(" — pinned: {fam} (themes keep it)")
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

    /// The installed family answering the first entry in `prefs` the machine
    /// has, matched by TYPEFACE rather than by exact name.
    ///
    /// Themes state a preference, not a font: a family that isn't installed
    /// makes fontdb substitute a proportional face and cell rounding then
    /// mangles every glyph. `None` = none of them are here, and the caller
    /// must leave the font alone rather than guess.
    ///
    /// Matching by key is what keeps that promise now the pool holds one
    /// spelling per typeface (`fontlist::one_per_typeface`). A machine with
    /// `Lilex Nerd Font` installed lists it INSTEAD of plain `Lilex` — and
    /// every preference list ends in plain `Lilex`, the face crew embeds, so
    /// an exact match would have resolved nothing there and left the theme's
    /// font unset on exactly the machines that had the better copy.
    pub(crate) fn resolve_family(&mut self, prefs: &[&str]) -> Option<String> {
        let pool = self.font_pool();
        prefs.iter().find_map(|want| {
            let key = crew_theme::typeface_key(want);
            pool.iter()
                .find(|have| crew_theme::typeface_key(have) == key)
                .cloned()
        })
    }
}

/// Keep only the typefaces in [`crew_theme::FONT_ALLOWLIST`]. If a machine has
/// none of them installed, fall back to the full set so the app still has a
/// font rather than none at all.
///
/// Matched by [`crew_theme::typeface_key`], not by exact name: the allowlist
/// names one spelling per face and the machine offers whichever it has, so
/// `Comic Mono` on the list answers for an installed `ComicMono Nerd Font
/// Mono`. Spelled-out matching meant a face was in the pool only under the
/// names somebody had remembered to write down.
pub(crate) fn allowed_pool(installed: Vec<String>) -> Vec<String> {
    let keys: Vec<String> = crew_theme::FONT_ALLOWLIST
        .iter()
        .map(|f| crew_theme::typeface_key(f))
        .collect();
    let allowed: Vec<String> = installed
        .iter()
        .filter(|f| keys.contains(&crew_theme::typeface_key(f)))
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
