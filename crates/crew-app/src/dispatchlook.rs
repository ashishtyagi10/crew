//! The appearance commands that switch a LOOK on or off — the CRT tube, the
//! glass sheet, marks, invisibles, motion and shape cues.
//!
//! Split out of [`crate::dispatch`] for the line cap. Its siblings
//! [`crate::dispatchtype`] and [`crate::dispatchspace`] hold the ones that
//! take a NUMBER, which is the line this split follows.
use crate::app::CrewApp;

#[cfg(test)]
#[path = "dispatchlook_tests.rs"]
mod tests;

impl CrewApp {
    /// Push both glass settings into the renderer, and keep the window's own
    /// opaque flag in step with them.
    ///
    /// The window half is not optional bookkeeping: the renderer only controls
    /// the alpha of the frame *crew* draws, while the OS-drawn title bar
    /// composites against `NSWindow.isOpaque`. Setting one without the other is
    /// what left the title bar see-through at full opacity.
    pub(crate) fn apply_glass(&mut self) {
        let level = self.config.glass_level();
        let opacity = self.config.window_opacity;
        if let Some(r) = &mut self.renderer {
            r.set_glass(level);
            r.set_window_opacity(opacity);
        }
        if let Some(w) = &self.window {
            w.set_transparent(crate::config::wants_window_transparency(opacity));
        }
    }

    /// Handle `/crt [on|off|auto]`: force the CRT tube post-process on or off, or
    /// (`auto`) follow the theme's own `crt` flag. Bare `/crt` toggles the
    /// current effective state into an explicit override. Persisted; the
    /// renderer reads the effective state every frame, so a redraw applies it.
    pub(crate) fn crt_command(&mut self, arg: &str) {
        let msg = match arg {
            "" => {
                // Toggle: pin the opposite of what's showing now.
                let next = self.effective_crt().is_none();
                self.config.crt = Some(next);
                if next {
                    "CRT on"
                } else {
                    "CRT off"
                }
            }
            "on" => {
                self.config.crt = Some(true);
                "CRT on"
            }
            "off" => {
                self.config.crt = Some(false);
                "CRT off"
            }
            "auto" => {
                self.config.crt = None;
                if self.effective_crt().is_some() {
                    "CRT auto (on for this theme)"
                } else {
                    "CRT auto (off for this theme)"
                }
            }
            _ => {
                self.set_status("usage: /crt [on|off|auto]");
                return;
            }
        };
        self.config.save();
        self.set_status(msg);
        self.redraw();
    }

    /// `/motion [auto|off|subtle|full]` — how much crew moves.
    ///
    /// `auto` defers to the OS accessibility switch, which is where a user who
    /// wants less motion has almost certainly already said so. A bare
    /// `/motion` reports the preference AND what it currently resolves to,
    /// because "auto" alone does not answer the question the user asked.
    /// `/marks [on|off]` — the ticks and bars pane cards draw on their
    /// borders. No argument reports the current setting, like every other
    /// look switch.
    pub(crate) fn marks_command(&mut self, arg: &str) {
        let state = |on: bool| if on { "on" } else { "off" };
        if arg.trim().is_empty() {
            self.set_status(format!(
                "card border marks {} (/marks [on|off])",
                state(self.config.border_marks)
            ));
            return;
        }
        let Some(on) = crate::bordermarks::parse(arg) else {
            self.set_status("usage: /marks [on|off]");
            return;
        };
        self.config.border_marks = on;
        self.config.save();
        crate::bordermarks::set(on);
        self.set_status(format!("card border marks {}", state(on)));
    }

    /// `/invisibles [on|off]` — reveal the characters that say something
    /// without printing anything, in the file viewer. Tabs are always
    /// EXPANDED; this only decides whether they are also marked.
    pub(crate) fn invisibles_command(&mut self, arg: &str) {
        let state = |on: bool| if on { "on" } else { "off" };
        if arg.trim().is_empty() {
            self.set_status(format!(
                "invisibles {} (/invisibles [on|off])",
                state(self.config.invisibles)
            ));
            return;
        }
        // The same two-word vocabulary `/marks` takes: two switches that
        // answered "on"/"off" differently would be two switches to remember.
        let Some(on) = crate::bordermarks::parse(arg) else {
            self.set_status("usage: /invisibles [on|off]");
            return;
        };
        self.config.invisibles = on;
        self.config.save();
        crate::invisibles::set(on);
        self.set_status(format!("invisibles {}", state(on)));
        self.redraw();
    }

    pub(crate) fn motion_command(&mut self, arg: &str) {
        use crate::motion::MotionPref;
        if arg.is_empty() {
            let reduce = crate::motion::os_reduce();
            let os = if reduce {
                "; the OS asks for reduced motion"
            } else {
                ""
            };
            self.set_status(format!(
                "motion {}{os} (/motion [auto|off|subtle|full])",
                self.config.motion_pref().label(reduce)
            ));
            return;
        }
        let Some(pref) = MotionPref::parse(arg) else {
            self.set_status("usage: /motion [auto|off|subtle|full]");
            return;
        };
        self.config.motion = pref.as_str().to_string();
        self.config.save();
        crate::motion::set_level(self.config.motion_level());
        self.set_status(format!("motion {}", pref.label(crate::motion::os_reduce())));
        self.redraw();
    }

    /// `/shapes [auto|off|on]` — say it with a shape as well as a colour.
    pub(crate) fn shapes_command(&mut self, arg: &str) {
        const ALL: [&str; 3] = ["auto", "off", "on"];
        let os = crate::shapecues::os_asks();
        if arg.is_empty() {
            let now = if self.config.shape_cues(os) {
                "on"
            } else {
                "off"
            };
            self.set_status(format!(
                "shape cues {} ({now}) (/shapes [auto|off|on])",
                self.config.shape_cues
            ));
            return;
        }
        let arg = arg.trim().to_ascii_lowercase();
        if !ALL.contains(&arg.as_str()) {
            self.set_status("usage: /shapes [auto|off|on]");
            return;
        }
        self.config.shape_cues = arg;
        self.config.save();
        let on = self.config.shape_cues(os);
        crate::shapecues::set(on);
        self.set_status(format!(
            "shape cues {} ({})",
            self.config.shape_cues,
            if on { "on" } else { "off" }
        ));
        self.redraw();
    }
}
