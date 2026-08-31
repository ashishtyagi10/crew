//! The type knobs that take a NUMBER — weight, smoothing and gamma — each
//! parsing a named step or a raw value and clamping it. How the GLYPH is drawn.
//!
//! Split out of [`crate::dispatch`] for the line cap; [`crate::dispatchlook`]
//! holds the on/off looks and [`crate::dispatchspace`] the knobs that move
//! type around rather than reshape it.
use crate::app::CrewApp;

#[cfg(test)]
#[path = "dispatchtype_tests.rs"]
mod tests;

impl CrewApp {
    /// Handle `/weight [<name>|<300-900>]`: set the base text weight so the font
    /// reads thicker or lighter. Accepts named steps (thin/normal/medium/
    /// semibold/bold/black) or a raw CSS number. Bare `/weight` reports the
    /// current value. Persisted and applied live.
    pub(crate) fn weight_command(&mut self, arg: &str) {
        let named = |a: &str| -> Option<u16> {
            Some(match a {
                "thin" | "light" => 300,
                "normal" | "regular" => 400,
                "medium" => 500,
                "semibold" | "semi" => 600,
                "bold" => 700,
                "black" | "heavy" => 900,
                _ => return None,
            })
        };
        let weight = match arg {
            "" => {
                self.set_status(format!(
                    "font weight {} (/weight [thin|normal|medium|semibold|bold|black|<300-900>])",
                    self.config.font_weight
                ));
                return;
            }
            a => match named(a).or_else(|| a.parse::<u16>().ok()) {
                Some(w) => w.clamp(300, 900),
                None => {
                    self.set_status(
                        "usage: /weight [thin|normal|medium|semibold|bold|black|<300-900>]",
                    );
                    return;
                }
            },
        };
        self.config.font_weight = weight;
        self.config.save();
        if let Some(r) = &mut self.renderer {
            r.set_font_weight(Some(weight));
        }
        self.set_status(format!("font weight {weight}"));
        self.redraw();
    }

    /// Handle `/smooth [off|light|medium|heavy|<0-255>]`: set the CoreText-style
    /// font smoothing strength (stem darkening — how full the glyphs read).
    /// Bare `/smooth` reports the current value. Persisted and applied live.
    /// The keyword ladder is `smoothlvl` — shared with the Settings form's
    /// Smoothing picker, so the two surfaces can never disagree.
    pub(crate) fn smooth_command(&mut self, arg: &str) {
        let named = crate::smoothlvl::strength_of;
        let strength = match arg {
            "" => {
                self.set_status(format!(
                    "font smoothing {} (/smooth [off|light|medium|heavy|<0-255>])",
                    self.config.font_smooth
                ));
                return;
            }
            a => match named(a).or_else(|| a.parse::<u16>().ok().map(|s| s.min(255) as u8)) {
                Some(s) => s,
                None => {
                    self.set_status("usage: /smooth [off|light|medium|heavy|<0-255>]");
                    return;
                }
            },
        };
        self.config.font_smooth = strength;
        self.config.save();
        if let Some(r) = &mut self.renderer {
            r.set_text_smoothing(Some(strength));
        }
        self.set_status(format!("font smoothing {strength}"));
        self.redraw();
    }

    /// Handle `/gamma [off|light|medium|full|<0-255>]`: how much of the
    /// encoded blend's gamma error the coverage curve takes back. Crew blends
    /// text on gamma-encoded values, so a half-covered edge pixel emits about
    /// a fifth of the light it should — light ink on a dark page reads thin
    /// and dark ink on a bright one reads blotted. Bare `/gamma` reports the
    /// current amount. Persisted and applied live. The keyword ladder is
    /// `gammalvl` — shared with the Settings form's Text gamma picker, so the
    /// two surfaces can never disagree.
    pub(crate) fn gamma_command(&mut self, arg: &str) {
        let named = crate::gammalvl::amount_of;
        let amount = match arg {
            "" => {
                self.set_status(format!(
                    "text gamma {} (/gamma [off|light|medium|full|<0-255>])",
                    self.config.font_gamma
                ));
                return;
            }
            a => match named(a).or_else(|| a.parse::<u16>().ok().map(|s| s.min(255) as u8)) {
                Some(a) => a,
                None => {
                    self.set_status("usage: /gamma [off|light|medium|full|<0-255>]");
                    return;
                }
            },
        };
        self.config.font_gamma = amount;
        self.config.save();
        if let Some(r) = &mut self.renderer {
            r.set_text_gamma(Some(amount));
        }
        self.set_status(format!("text gamma {amount}"));
        self.redraw();
    }
}
