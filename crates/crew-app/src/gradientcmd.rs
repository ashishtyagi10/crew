//! `/gradient` — the one place the canvas's colour is answered for.
//!
//! Crew draws every gradient surface (the page's wash, the dot lattice, every
//! card's stroke, the footer meters) between two poles. This command owns both
//! halves of that: how far the pair is allowed to *lean* over time (the
//! [`crate::gradientlvl`] ladder, also in Settings), and — new here — whether
//! the pair is the theme's own or a pair the user picked.
//!
//! ```text
//! /gradient                     what it is now, and how to change it
//! /gradient off|subtle|lively   how far the colour breathes
//! /gradient #7aa2f7 #bb9af7     a gradient of your own
//! /gradient reset               back to the theme's own poles
//! ```
//!
//! **Only the hue and chroma of a custom pair are the user's.** The lightness
//! stays the theme's, applied at read time so it tracks the ten-minute palette
//! rotation. The wash lies under the text with 4-16% contrast headroom over
//! the page it lifts, and a pole a few steps brighter would spend headroom
//! that is not there — `#ffffff` would erase the page. So the user chooses the
//! colour and crew chooses how bright it is (crew-theme's
//! `poleshift::relight`, and the sweep in `poleshift_tests` that holds every
//! hue on every page above the WCAG non-text floor).
use crew_theme::poleshift::Poles;

use crate::app::CrewApp;
use crate::gradientlvl::GradientLevel;
use crate::palette::parse_hex;

/// Two `#rrggbb` colours separated by whitespace or a comma. `None` unless
/// there are exactly two and both parse — a half-understood gradient would
/// leave the canvas in a state nobody asked for.
pub(crate) fn parse_poles(arg: &str) -> Option<Poles> {
    let mut parts = arg
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty());
    let a = parse_hex(parts.next()?)?;
    let b = parse_hex(parts.next()?)?;
    parts.next().is_none().then_some((a, b))
}

/// The stored form of a pair: what `/gradient` writes to the config and what
/// [`parse_poles`] reads back.
pub(crate) fn format_poles(((ar, ag, ab), (br, bg, bb)): Poles) -> String {
    format!("#{ar:02x}{ag:02x}{ab:02x} #{br:02x}{bg:02x}{bb:02x}")
}

impl CrewApp {
    /// Push the gradient config — the ladder and the custom pair — into the
    /// live globals every gradient surface reads.
    ///
    /// One function so Save, session restore, an external config edit and the
    /// command itself can never disagree; called from `apply_config`.
    pub(crate) fn apply_gradient(&self) {
        let level = self.config.gradient_level();
        crate::gradientlvl::set_level(level);
        // Off must also put the poles back where they were: the shift is a
        // live global, and left where the last breath stopped it the canvas
        // would keep wearing a colour the setting says it no longer does.
        if level == GradientLevel::Off {
            crew_theme::poleshift::set_shift(0.0);
        }
        crew_theme::poleshift::set_custom(
            self.config.gradient_poles.as_deref().and_then(parse_poles),
        );
    }

    /// `/gradient [off|subtle|lively | <#a> <#b> | reset]`.
    pub(crate) fn gradient_command(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            let level = self.config.gradient_level();
            let poles = match crew_theme::poleshift::poles() {
                Some(p) => format_poles(p),
                None => "none".to_string(),
            };
            let own = if self.config.gradient_poles.is_some() {
                "yours"
            } else {
                "the theme's"
            };
            self.set_status(format!(
                "gradient {} — {own} poles {poles} (/gradient [off|subtle|lively|<#a> <#b>|reset])",
                level.as_str()
            ));
            return;
        }
        if arg.eq_ignore_ascii_case("reset") || arg.eq_ignore_ascii_case("default") {
            self.config.gradient_poles = None;
            self.commit_gradient("gradient back to the theme's own poles");
            return;
        }
        if let Some(level) = GradientLevel::parse(arg) {
            self.config.gradient = level.as_str().to_string();
            let how = match level {
                GradientLevel::Off => "fixed to the theme's colour",
                GradientLevel::Subtle => "breathing gently",
                GradientLevel::Lively => "breathing wide",
            };
            self.commit_gradient(&format!("gradient {} — {how}", level.as_str()));
            return;
        }
        let Some(poles) = parse_poles(arg) else {
            self.set_status("usage: /gradient [off|subtle|lively|<#rrggbb> <#rrggbb>|reset]");
            return;
        };
        self.config.gradient_poles = Some(format_poles(poles));
        self.apply_gradient();
        // Report what will actually be DRAWN, not what was typed: the
        // lightness is the theme's, so a pair typed as `#ffffff` comes back at
        // the page's own brightness, and saying otherwise would look like a
        // bug the first time someone tried it.
        let shown =
            crew_theme::poleshift::poles().map_or_else(|| format_poles(poles), format_poles);
        self.commit_gradient(&format!(
            "gradient {shown} — your hue, the theme's brightness"
        ));
    }

    /// Save, re-apply and repaint after a `/gradient` change.
    fn commit_gradient(&mut self, note: &str) {
        self.config.save();
        self.apply_gradient();
        self.set_status(note.to_string());
        self.redraw();
    }
}

#[cfg(test)]
#[path = "gradientcmd_tests.rs"]
mod tests;
