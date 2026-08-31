//! The knobs that move type around rather than reshape it — density (the gap
//! between cards), leading (the gap between lines) and contrast (how far the
//! ink sits from the page).
//!
//! Split from [`crate::dispatchtype`] for the line cap, along the line between
//! drawing a glyph and placing one.
use crate::app::CrewApp;

impl CrewApp {
    /// `/density [compact|cozy|roomy]` — how tightly crew packs the canvas.
    ///
    /// Applies live: the gutter and the chat spacers are both read out of the
    /// atomic at layout time, so there is nothing to rebuild.
    pub(crate) fn density_command(&mut self, arg: &str) {
        use crate::density::Density;
        if arg.is_empty() {
            self.set_status(format!(
                "density {} (/density [compact|cozy|roomy])",
                self.config.density().as_str()
            ));
            return;
        }
        let Some(d) = Density::parse(arg) else {
            self.set_status("usage: /density [compact|cozy|roomy]");
            return;
        };
        self.config.density = d.as_str().to_string();
        self.config.save();
        crate::density::set_level(d);
        self.set_status(format!("density {}", d.as_str()));
        self.redraw();
    }

    /// `/leading [tight|normal|relaxed|loose]` — how much air sits between
    /// rows of text (see [`crate::leading`]).
    ///
    /// Changing it changes the CELL, so the pane grid is remeasured and every
    /// PTY is resized: a shell that thought it had 24 rows has to be told it
    /// now has 21, or it will keep drawing off the bottom of its card. That
    /// is what `apply_config` already does for a font-size change, and this
    /// rides the same path rather than a second one.
    pub(crate) fn leading_command(&mut self, arg: &str) {
        use crate::leading::Leading;
        if arg.is_empty() {
            self.set_status(format!(
                "leading {} (/leading [tight|normal|relaxed|loose])",
                self.config.leading().as_str()
            ));
            return;
        }
        let Some(l) = Leading::parse(arg) else {
            self.set_status("usage: /leading [tight|normal|relaxed|loose]");
            return;
        };
        self.config.leading = l.as_str().to_string();
        self.config.save();
        let cfg = self.config.clone();
        self.apply_config(cfg);
        self.set_status(format!("leading {}", l.as_str()));
        self.redraw();
    }

    /// `/contrast [auto|normal|high]` — the WCAG floor every derived colour
    /// is measured against.
    ///
    /// `auto` defers to the OS accessibility switch, which is where a user who
    /// wants more contrast has almost certainly already said so. A bare
    /// `/contrast` reports the setting AND what it currently resolves to.
    pub(crate) fn contrast_command(&mut self, arg: &str) {
        const ALL: [&str; 3] = ["auto", "normal", "high"];
        let os = crate::oscontrast::increase_contrast();
        if arg.is_empty() {
            let band = if self.config.high_contrast(os) {
                "AAA"
            } else {
                "AA"
            };
            self.set_status(format!(
                "contrast {} \u{2014} WCAG {band} floors (/contrast [auto|normal|high])",
                self.config.contrast
            ));
            return;
        }
        let arg = arg.trim().to_ascii_lowercase();
        if !ALL.contains(&arg.as_str()) {
            self.set_status("usage: /contrast [auto|normal|high]");
            return;
        }
        self.config.contrast = arg;
        self.config.save();
        let high = self.config.high_contrast(os);
        crew_theme::contrast::set_high_contrast(high);
        self.set_status(format!(
            "contrast {} \u{2014} WCAG {} floors",
            self.config.contrast,
            if high { "AAA" } else { "AA" }
        ));
        self.redraw();
    }
}
