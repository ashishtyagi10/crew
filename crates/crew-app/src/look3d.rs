//! `/look 3d` — a 3D scene behind the panes (`crew_render::stage3d`), lit
//! in the theme's own colours, and the clock it turns on.
//!
//! ```text
//! /look 3d          what it is set to, and the scenes there are
//! /look 3d orbit    a metal knot with a ring of crystals going round it
//! /look 3d off      none (the default)
//! ```
//!
//! Its clock is its own, not the gradient's wash: the wash turns only on the
//! themes with a drift period, and the scene should move on every theme.
//! Like the wash it turns only while a pane works, on frames that work was
//! already drawing, and holds at Motion off, so an idle window costs nothing.
use crew_render::stage3d::{self, Stage};

use crate::app::CrewApp;

/// Ms per turn of the scene while a pane works: slow enough to be a mood.
pub(crate) const TURN_MS: u64 = 40_000;

/// A saved setting as it is kept: a scene's name, or empty for none.
pub(crate) fn clean(s: String) -> String {
    match Stage::parse(s.trim()) {
        Some(st) => st.name().to_string(),
        None => String::new(),
    }
}

/// The picker's rows for `/look 3d `.
pub(crate) fn ladder() -> Vec<(String, String)> {
    let mut rows = vec![(
        "off".to_string(),
        "no scene \u{2014} the default".to_string(),
    )];
    rows.extend(Stage::ALL.map(|s| (s.name().to_string(), about(s).to_string())));
    rows
}

/// What to add when the live theme cannot show it: a paper page is solid,
/// so the scene shows only in the gutters until the theme is glass.
fn solid_note() -> &'static str {
    match crew_theme::theme().liquid {
        Some(_) => " \u{2014} it turns while a pane works",
        None => " \u{2014} this theme's panes are solid; it shows behind a glass theme",
    }
}

fn about(s: Stage) -> &'static str {
    match s {
        Stage::Orbit => "a metal knot with a ring of crystals going round it",
    }
}

impl CrewApp {
    /// `/look 3d [<scene>|off]`. Persisted; the next frame draws it.
    pub(crate) fn backdrop_3d_command(&mut self, arg: &str) {
        let names: Vec<&str> = Stage::ALL.iter().map(|s| s.name()).collect();
        let msg = match arg {
            "" => format!(
                "3D backdrop: {} \u{2014} /look 3d <{}|off>",
                Stage::parse(&self.config.backdrop_3d).map_or("off", Stage::name),
                names.join("|")
            ),
            "off" | "none" => {
                self.config.backdrop_3d.clear();
                "3D backdrop off".to_string()
            }
            name => match Stage::parse(name) {
                Some(s) => {
                    self.config.backdrop_3d = s.name().to_string();
                    format!("3D backdrop: {}{}", s.name(), solid_note())
                }
                None => {
                    self.set_status(format!("usage: /look 3d <{}|off>", names.join("|")));
                    return;
                }
            },
        };
        if !arg.is_empty() {
            self.config.save();
            self.redraw();
        }
        self.set_status(msg);
    }

    /// This frame's scene and where it is: called once per drawn frame.
    pub(crate) fn publish_stage_3d(&mut self, busy: bool) {
        let s = Stage::parse(&self.config.backdrop_3d);
        stage3d::set_stage(s);
        if s.is_some() {
            let pace = busy.then_some(TURN_MS);
            let now = crate::anim::now_ms();
            stage3d::set_phase(self.stage_clock.advance(now, pace, crate::motion::level()));
        }
    }
}

#[cfg(test)]
#[path = "look3d_tests.rs"]
mod tests;
