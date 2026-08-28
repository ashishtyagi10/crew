//! `/opacity` — how much of the desktop crew lets through.
//!
//! ```text
//! /opacity            what it is now, and how to change it
//! /opacity off        solid again (100%)
//! /opacity subtle     97% — a breath of desktop, no more
//! /opacity medium     93%
//! /opacity sheer      88% — as far as the named steps go
//! /opacity 60         any percent down to the floor (35%)
//! ```
//!
//! **The named steps are deliberately shy.** Translucency is a texture, not a
//! window into the wallpaper: past a tenth or so of desktop the canvas starts
//! competing with the work on it, and the ladder is where a first `/opacity`
//! lands. Someone who wants the aquarium look can still type the number.
//!
//! The knob is the one Settings has always had (WINDOW → Opacity %); this is
//! the same value, reachable from the input bar and applied live.
//!
//! **What goes sheer is the page, not the work.** The window's alpha rides the
//! page colour, so cell backgrounds and text blend on top of it and stay
//! solid, and the card you are reading — with the input bar and the nav, which
//! are crew's furniture rather than scenery — is solidified outright
//! (crew-render's `solidcard`, fed by [`crate::chrome::solid_chrome`]). The
//! desktop shows through the canvas around your panes and through every card
//! you are NOT reading.
use crate::app::CrewApp;
use crate::config::MIN_WINDOW_OPACITY;

/// The ladder the value picker offers, sheerest last. Named steps exist so
/// `/opacity medium` is a choice you can make without picking a number out of
/// the air; any percent still works.
pub(crate) const LADDER: &[(&str, f32, &str)] = &[
    ("off", 1.0, "solid — no desktop at all"),
    ("subtle", 0.97, "97% — a breath of desktop, no more"),
    ("medium", 0.93, "93% — the default when you ask for glass"),
    ("sheer", 0.88, "88% — as far as the named steps go"),
];

/// Parse an opacity argument into a window alpha.
///
/// Accepts a ladder name, a percent (`85`, `85%`) or a fraction (`0.85`) —
/// people write it all three ways, and the three cannot collide: no fraction
/// of a window is above 1, and no percent is below it.
///
/// Out-of-range numbers CLAMP rather than fail: the floor exists so a window
/// can never be dialled away entirely ([`MIN_WINDOW_OPACITY`]), and refusing
/// `/opacity 10` would teach nothing while doing nothing. The caller says what
/// was actually applied.
pub(crate) fn parse(arg: &str) -> Option<f32> {
    let arg = arg.trim().to_ascii_lowercase();
    if let Some((_, v, _)) = LADDER.iter().find(|(n, _, _)| *n == arg) {
        return Some(*v);
    }
    // `on` is the friendly alias for "some glass, you pick how much" — the
    // same word `/glass on` takes, and it means the ladder's middle.
    if arg == "on" {
        return Some(0.93);
    }
    let n: f32 = arg.trim_end_matches('%').parse().ok()?;
    if !n.is_finite() {
        return None;
    }
    let frac = if n <= 1.0 { n } else { n / 100.0 };
    Some(frac.clamp(MIN_WINDOW_OPACITY, 1.0))
}

/// An opacity as the user talks about it: whole percent.
pub(crate) fn percent(opacity: f32) -> String {
    format!("{}%", (opacity * 100.0).round() as i32)
}

impl CrewApp {
    /// Run `/opacity [off|subtle|medium|sheer|<35-100>]`. Persisted, and
    /// applied to the live window through the same path Settings uses.
    pub(crate) fn opacity_command(&mut self, arg: &str) {
        if arg.is_empty() {
            let now = self.config.window_opacity;
            self.set_status(format!(
                "opacity {} (/opacity [off|subtle|medium|sheer|<35-100>])",
                percent(now)
            ));
            return;
        }
        let Some(o) = parse(arg) else {
            self.set_status("usage: /opacity [off|subtle|medium|sheer|<35-100>]");
            return;
        };
        self.config.window_opacity = o;
        self.config.save();
        // Both halves of the switch: the renderer's alpha and the window's own
        // opaque flag (see `apply_glass` — setting one without the other is
        // what left the title bar see-through at full opacity).
        self.apply_glass();
        self.set_status(match o >= 1.0 {
            true => "opacity 100% — solid".to_string(),
            false => format!(
                "opacity {} — the card you are reading, the input bar and the nav stay solid",
                percent(o)
            ),
        });
        self.redraw();
    }
}

#[cfg(test)]
#[path = "opacitycmd_tests.rs"]
mod tests;
