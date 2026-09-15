//! One-shot upgrade heals: config a new default has to reach back and fix.
//!
//! Split from `config.rs` (child module) along the line between what a
//! setting IS and what an older spelling of it has to become. Each of these
//! fires once, only on a value the user never chose themselves, and says so
//! by returning true — a heal that cannot tell "the old default" from "their
//! answer" would be overwriting taste.
use super::*;

impl CrewConfig {
    /// One-shot upgrade heal: a config still carrying the pre-gamma default
    /// takes the rebalanced one, because `/gamma` now does the half of the
    /// job that strength was silently doing and the two together overshoot.
    /// A strength the user actually chose is left alone. Returns true when
    /// anything changed.
    pub fn adopt_rebalanced_smoothing(&mut self) -> bool {
        if self.font_smooth == SMOOTH_BEFORE_GAMMA {
            self.font_smooth = crew_render::DEFAULT_SMOOTH;
            return true;
        }
        false
    }

    /// One-shot upgrade heal for 0.19.62: a config still carrying the pair
    /// the 0.19.28 rebalance left behind takes the undilated one. Swept over
    /// eight glyphs at two sizes, that pair delivered 98% of the outline's
    /// light on a dark page but **145%** on a bright one, and needed 45% more
    /// inked pixels to do it; the curve alone lands on 100% both ways up.
    ///
    /// Both keys must still be at their old defaults — someone who chose a
    /// strength, or an amount, chose the pair, and neither half moves under
    /// them. Returns true when anything changed.
    pub fn adopt_undilated_text(&mut self) -> bool {
        if self.font_smooth != SMOOTH_AFTER_GAMMA || self.font_gamma != GAMMA_WITH_DILATION {
            return false;
        }
        self.font_smooth = crew_render::DEFAULT_SMOOTH;
        self.font_gamma = crew_render::DEFAULT_TEXT_GAMMA;
        true
    }
}

#[cfg(test)]
#[path = "configmigrate_tests.rs"]
mod tests;
