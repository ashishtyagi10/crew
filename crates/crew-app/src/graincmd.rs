//! `/grain` — how much newsprint texture the page carries.
//!
//! ```text
//! /grain            what it is now, and how to change it
//! /grain off        a flat page — no texture at all
//! /grain light      half the default: a suggestion of paper
//! /grain medium     the default newsprint
//! /grain heavy      as far as the named steps go
//! /grain 0.4        any amount from 0 to 2
//! ```
//!
//! The grain is a deliberate part of crew's look — a per-pixel hash plus a
//! coarser fibre octave, calibrated so the near-black "newspaper" pages and
//! the bright "paper" ones carry the same texture (the arbiter is
//! `grain_is_newsprint_on_every_theme` in `crew-theme`, and the amplitude
//! itself is measured by the headless paperbg harness). At the default it is
//! a standard deviation of about six levels on a dark page.
//!
//! Six levels is a lot of texture next to a terminal that has none, and
//! whether that reads as *paper* or as *noise* is taste, not a bug — but it
//! was taste you could only exercise by opening `/settings` and finding a
//! numeric field called "Grain (0-2)". Every other look knob crew has —
//! `/smooth`, `/gamma`, `/weight`, `/leading`, `/opacity`, `/density` — is a
//! named ladder you can type at the input bar and see change under you. This
//! is the same knob those all are: one `paper_grain` key, shared with the
//! Settings field, applied live.
use crate::app::CrewApp;

/// The named steps, flattest first. `medium` is the calibrated default; the
/// others are simple fractions and multiples of it, so the ladder says
/// "less" and "more" rather than inventing four separate calibrations.
pub(crate) const LADDER: &[(&str, f32, &str)] = &[
    ("off", 0.0, "flat — no texture at all"),
    ("light", 0.6, "half the newsprint"),
    ("medium", 1.3, "the default newsprint"),
    ("heavy", 1.8, "as far as the named steps go"),
];

/// Largest amount the knob accepts — the same clamp `clamped()` applies, so
/// the command and a hand-edited config agree.
pub(crate) const MAX: f32 = 2.0;

/// Parse a `/grain` argument: a ladder name or a number in `0..=2`.
/// Out-of-range numbers clamp rather than fail, the same way `/opacity` does
/// — refusing `/grain 9` would teach nothing while doing nothing.
pub(crate) fn parse(arg: &str) -> Option<f32> {
    let arg = arg.trim().to_ascii_lowercase();
    if let Some((_, v, _)) = LADDER.iter().find(|(n, _, _)| *n == arg) {
        return Some(*v);
    }
    // `on` means "the texture crew ships with" — the ladder's middle, and the
    // same word `/opacity on` and `/glass on` take.
    if arg == "on" {
        return Some(crate::config::default_paper_grain());
    }
    let n: f32 = arg.parse().ok()?;
    n.is_finite().then(|| n.clamp(0.0, MAX))
}

/// Display label for an amount: the keyword when it sits on the ladder, the
/// number otherwise — a custom `/grain 0.4` shows as `0.4`, not a near name.
pub(crate) fn label_of(amount: f32) -> String {
    LADDER
        .iter()
        .find(|(_, v, _)| (v - amount).abs() < 1e-4)
        .map(|(n, _, _)| n.to_string())
        .unwrap_or_else(|| format!("{amount:.1}"))
}

impl CrewApp {
    /// Run `/grain [off|light|medium|heavy|<0-2>]`. Persisted, and applied to
    /// the live page through the same setter Settings uses.
    pub(crate) fn grain_command(&mut self, arg: &str) {
        if arg.is_empty() {
            self.set_status(format!(
                "grain {} (/grain [off|light|medium|heavy|<0-2>])",
                label_of(self.config.paper_grain)
            ));
            return;
        }
        let Some(g) = parse(arg) else {
            self.set_status("usage: /grain [off|light|medium|heavy|<0-2>]");
            return;
        };
        self.config.paper_grain = g;
        self.config.save();
        if let Some(r) = &mut self.renderer {
            r.set_paper_grain(g);
        }
        self.set_status(format!("grain {}", label_of(g)));
        self.redraw();
    }
}

#[cfg(test)]
#[path = "graincmd_tests.rs"]
mod tests;
