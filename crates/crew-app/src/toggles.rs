//! Palette-discoverable toggles that mirror the Cmd-chord shortcuts: `/broadcast`
//! (Cmd+S), `/zoom` (Cmd+Z), `/sidebar` (Cmd+G). The fuzzy command palette
//! surfaces them by name when you can't recall the chord. The chords call the
//! same methods, so behaviour stays in lockstep.
use crate::app::CrewApp;
use crate::chords::broadcast_label;

impl CrewApp {
    /// Toggle broadcast — mirror typed input to every terminal pane.
    pub(crate) fn toggle_broadcast(&mut self) {
        self.broadcast = !self.broadcast;
        self.input.broadcast = self.broadcast;
        self.set_status(broadcast_label(self.broadcast));
        self.redraw();
    }

    /// Advance the theme cycle (Ctrl+Shift+L): `dark` → `light` → `crt`,
    /// wrapping — so the one hotkey reaches all three consolidated themes,
    /// persists the choice, and repaints exactly like the `/theme` command.
    pub(crate) fn toggle_theme(&mut self) {
        let label = crew_theme::cycle_next(crate::chattime::unix_now_ms());
        self.config.theme = Some(label.to_string());
        crate::palette::set_accent(self.config.accent_rgb());
        self.config.save();
        self.redraw();
        self.set_status(format!("theme: {label}"));
    }

    /// Toggle zoom — the focused pane fills the content area.
    /// Focus mode on/off (`/focus`). Entering resets the held count; leaving
    /// reports it as one card, so the mode's cost is bounded by a single
    /// line rather than by whatever you failed to notice.
    pub(crate) fn toggle_focus_mode(&mut self) {
        let on = !crate::focusmode::on();
        crate::focusmode::set(on);
        if on {
            self.held = crate::focusmode::Held::default();
            self.set_status("focus mode on \u{2014} nothing will interrupt (/focus to leave)");
        } else {
            let summary = std::mem::take(&mut self.held).summary();
            match summary {
                Some(line) => {
                    self.toasts
                        .push(line.clone(), "held", false, crate::anim::now_ms());
                    self.set_status(line);
                }
                None => self.set_status("focus mode off"),
            }
        }
        self.redraw();
    }

    pub(crate) fn toggle_zoom(&mut self) {
        // Remember where the pane was so the zoom can travel out of (and back
        // into) its own tile rather than cutting.
        self.zoom_from = self.panes.get(self.focused).map(|p| p.rect);
        self.zoom_anim =
            crate::ease::Timeline::start(crate::anim::now_ms(), 240, crate::motion::level());
        self.zoomed = !self.zoomed;
        self.set_status(if self.zoomed { "zoomed" } else { "unzoomed" });
        self.redraw();
    }
}

#[cfg(test)]
#[path = "toggles_tests.rs"]
mod tests;
