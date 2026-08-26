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
//! /gradient ember               one of the named pairs (crew-theme's `gradients`)
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

/// A named gradient off crew-theme's shelf, or two `#rrggbb` colours
/// separated by whitespace or a comma.
///
/// Names first: they are the discoverable form (the value picker lists them)
/// and no name is six hex digits, so the two forms cannot collide. `None`
/// unless the whole argument is understood — a half-read gradient would leave
/// the canvas in a state nobody asked for.
pub(crate) fn parse_poles(arg: &str) -> Option<Poles> {
    if let Some(p) = crew_theme::gradients::by_name(arg) {
        return Some(p);
    }
    let mut parts = arg
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty());
    let a = parse_hex(parts.next()?)?;
    let b = parse_hex(parts.next()?)?;
    parts.next().is_none().then_some((a, b))
}

/// The stored form of a pair: what `/gradient` writes to the config and what
/// [`parse_poles`] reads back.
///
/// A pair off the shelf is stored under its NAME, not its hex — so a config
/// says `gradient_poles = "ember"`, which is both readable and re-tunable:
/// improving a preset's colours then reaches everyone who chose it by name.
pub(crate) fn format_poles(poles: Poles) -> String {
    if let Some(name) = crew_theme::gradients::name_of(poles) {
        return name.to_string();
    }
    let ((ar, ag, ab), (br, bg, bb)) = poles;
    format!("#{ar:02x}{ag:02x}{ab:02x} #{br:02x}{bg:02x}{bb:02x}")
}

/// What to call a pair on screen: its name if it has one, otherwise the two
/// colours it will actually be drawn in.
fn describe(poles: Poles) -> String {
    crew_theme::gradients::name_of(poles).map_or_else(
        || {
            let ((ar, ag, ab), (br, bg, bb)) = poles;
            format!("#{ar:02x}{ag:02x}{ab:02x} #{br:02x}{bg:02x}{bb:02x}")
        },
        str::to_string,
    )
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

    /// Step to the next gradient on the shelf — `Ctrl+Shift+G`, the colour's
    /// answer to `Ctrl+Shift+L`.
    ///
    /// The walk passes back through the theme's OWN gradient once a lap, so
    /// the key that got you somewhere can always get you home.
    pub(crate) fn cycle_gradient(&mut self) {
        let next = crew_theme::gradients::next(crew_theme::poleshift::custom());
        self.config.gradient_poles = next.map(format_poles);
        let note = match next {
            Some(p) => format!("gradient {}", describe(p)),
            None => "gradient — the theme's own".to_string(),
        };
        self.commit_gradient(&note);
    }

    /// `/gradient [off|subtle|lively | <name> | <#a> <#b> | reset]`.
    pub(crate) fn gradient_command(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            let level = self.config.gradient_level();
            let poles = match crew_theme::poleshift::custom() {
                // Name what was CHOSEN, not what is drawn: the drawn pair has
                // been re-lit (and may be mid-breath), so its hex would never
                // match what the user typed.
                Some(p) => describe(p),
                None => crew_theme::poleshift::poles().map_or_else(|| "none".to_string(), describe),
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
            self.set_status(format!(
                "usage: /gradient [off|subtle|lively|{}|<#rrggbb> <#rrggbb>|reset]",
                crew_theme::gradients::GRADIENTS
                    .iter()
                    .map(|g| g.name)
                    .collect::<Vec<_>>()
                    .join("|")
            ));
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
