//! A CRT tube is a terminal running in glass, and its window is a TINTED
//! FACEPLATE: mostly dark, the desktop frosted faintly through it by the
//! window server's blur (`titlebar::apply_window`), the panes smoked glass on
//! top (crew-theme's `tube_glass`).
//!
//! It was a 12% veil for a day (2026-10-06, the user: "it should be almost
//! frosted glass, no black, like transparent"), and with a bright window
//! behind crew the desktop WAS the text's background: phosphor on white, the
//! glow a fog over it (the user, the same day: "this crt theme is ugly …
//! please have proper color contrast for font vs background"). A real tube's
//! faceplate is tinted for exactly that. [`TUBE_OPACITY`] is the least the
//! faceplate can be and hold every text role over a white desktop.
//!
//! Liquid glass is see-through too (2026-10-08, the user: "glass theme is not
//! glassy enough, I can't see the background"): its window is capped at the
//! theme's own `LiquidStyle::window`, the wallpaper a tint over the desktop.
//!
//! Everything that reads the window's opacity reads
//! [`CrewApp::window_opacity`], so the page, the blur, the title bar, the
//! solid overlays and the lifted borders all agree on it. A lower Opacity %
//! still wins — that is a choice of transparency over contrast, made in
//! Settings.
use crate::app::CrewApp;

/// How opaque a tube's faceplate is at most: enough that the phosphor holds
/// its contrast over a white window behind crew with no glass at all (see
/// `a_tube_holds_its_text_over_a_white_desktop` — 0.8 left the amber and
/// violet terminal text just under 7:1), while a sixth of the frosted
/// desktop still shows through.
pub(crate) const TUBE_OPACITY: f32 = 0.84;

/// The window's opacity for a `setting` under theme `t`: capped at
/// [`TUBE_OPACITY`] under a tube and at its own `window` under liquid glass.
pub(crate) fn sheer(setting: f32, t: &crew_theme::Theme) -> f32 {
    match t.liquid {
        _ if t.is_tube() => setting.min(TUBE_OPACITY),
        Some(l) => setting.min(l.window),
        None => setting,
    }
}

/// Who overrides an Opacity % of `setting` under `t`, and what the window
/// is instead: `("glass", 0.25)` on glass (whose cap is under the setting's
/// own floor, so always), `("tube", 0.84)` on a tube while its cap wins.
/// `None` when the setting is what the window gets.
pub(crate) fn overridden(setting: f32, t: &crew_theme::Theme) -> Option<(&'static str, f32)> {
    let got = sheer(setting, t);
    if got >= setting - 1e-3 {
        return None;
    }
    Some((if t.liquid.is_some() { "glass" } else { "tube" }, got))
}

impl CrewApp {
    /// The window's opacity this frame: the setting, capped under a tube or
    /// liquid glass ([`sheer`]).
    pub(crate) fn window_opacity(&self) -> f32 {
        sheer(self.config.window_opacity, crew_theme::theme())
    }

    /// Keep the window's sheer state in step with the theme. Run every frame
    /// (theme switches come from many paths), but it only reaches the
    /// renderer and AppKit when the opacity actually moves — a switch into or
    /// out of a tube.
    pub(crate) fn sync_window_opacity(&mut self) {
        let o = self.window_opacity();
        if self.applied_opacity == Some(o) {
            return;
        }
        self.applied_opacity = Some(o);
        self.apply_glass();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tube_and_glass_cap_their_opacity_and_other_themes_keep_the_setting() {
        use crew_theme::ThemeId;
        let tube = ThemeId::CrtGreen.theme();
        assert_eq!(sheer(1.0, tube), TUBE_OPACITY);
        // A lower Opacity % is transparency chosen over contrast: it wins.
        let low = crate::config::MIN_WINDOW_OPACITY;
        assert_eq!(sheer(low, tube), low);
        for id in [ThemeId::GlassSky, ThemeId::GlassDawn, ThemeId::GlassNight] {
            let t = id.theme();
            let cap = t.liquid.expect("glass is liquid").window;
            assert!(cap < 1.0, "{}: glass shows the desktop", id.as_str());
            assert_eq!(sheer(1.0, t), cap);
            // Glass is sheerer than the setting's own floor: nothing lower.
            assert_eq!(sheer(low, t), low.min(cap));
        }
        let paper = ThemeId::PaperDark.theme();
        assert_eq!(sheer(1.0, paper), 1.0);
        assert_eq!(sheer(0.6, paper), 0.6);
    }

    /// The text's background on a tube is the desktop through the faceplate,
    /// and the worst desktop is a white window behind crew. With no glass at
    /// all (the Off level), every text role still reads there: the terminal's
    /// text and ink at AAA, muted text at AA, and the quiet roles at the UI
    /// floor. At a 12% veil the ink was under 1.5:1 — the fog the user saw.
    #[test]
    fn a_tube_holds_its_text_over_a_white_desktop() {
        let mut under = Vec::new();
        for id in crew_theme::ALL_THEMES {
            let t = id.theme();
            if !t.is_tube() {
                continue;
            }
            // The window server composites premultiplied: the page at the
            // faceplate's alpha, plus what it leaves of the desktop.
            let c =
                |p: u8| (f32::from(p) * TUBE_OPACITY + 255.0 * (1.0 - TUBE_OPACITY)).round() as u8;
            let bg = (c(t.page_bg.0), c(t.page_bg.1), c(t.page_bg.2));
            for (role, fg, floor) in [
                ("term_fg", t.term_fg, 7.0),
                ("ink", t.ink, 7.0),
                ("text_muted", t.text_muted, 4.5),
                ("legend_off", t.legend_off, 3.0),
                ("hint_fg", t.hint_fg, 3.0),
                ("placeholder", t.placeholder, 3.0),
                ("dim", t.dim, 3.0),
            ] {
                let got = crew_theme::contrast_ratio(fg, bg);
                eprintln!("{}: {role} {got:.2} over {bg:?}", id.as_str());
                if got < floor {
                    under.push(format!("{} {role} {got:.2} (need {floor})", id.as_str()));
                }
            }
        }
        assert!(
            under.is_empty(),
            "over a white desktop:\n  {}",
            under.join("\n  ")
        );
        assert!(TUBE_OPACITY < 1.0, "a faceplate, not a wall");
    }

    #[test]
    fn the_app_reads_the_tube_cap_from_the_active_theme() {
        let _g = crate::app::theme_test_guard();
        let app = CrewApp::default();
        crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
        assert!(app.window_opacity() <= TUBE_OPACITY);
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        assert_eq!(app.window_opacity(), app.config.window_opacity);
    }
}
