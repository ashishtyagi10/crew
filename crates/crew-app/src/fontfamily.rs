//! The configured font family, applied — and refused out loud.
//!
//! The renderer never shapes a family that would not land on the cell grid
//! (`crew_render`'s `fontverify::usable`): one that is not installed makes
//! fontdb substitute a proportional face, whose spaces and digits take two
//! cells each, and the welcome's text slid half a window off its card
//! (2026-10-09, `font_family = "Intel One Mono"` on a Mac without it). It
//! draws the embedded face instead, and the app says why the font is not the
//! one in Settings.
use crate::app::CrewApp;

impl CrewApp {
    /// Push `config.font_family` to the renderer; when it cannot be drawn,
    /// the status line names it and the face drawn instead.
    pub(crate) fn apply_config_family(&mut self) {
        let Some(r) = &mut self.renderer else { return };
        let Some(asked) = crate::glyphs::apply_family(r, self.config.font_family.clone()) else {
            return;
        };
        let instead = r.font_family().unwrap_or(crew_theme::EMBEDDED_FAMILY);
        let note = format!("font {asked} is not installed (or not monospaced) — drawing {instead}");
        self.set_status(note);
    }
}
