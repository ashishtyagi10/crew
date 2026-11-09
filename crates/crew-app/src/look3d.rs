//! `/look 3d` — a 3D scene behind the panes (`crew_render::stage3d`), lit
//! in the theme's own colours, and the clock it turns on.
//!
//! ```text
//! /look 3d          what it is set to, and the scenes there are
//! /look 3d orbit    a metal knot with a ring of crystals going round it
//! /look 3d prism    glass prisms drifting in a fog of the page
//! /look 3d off      none, even on a theme that has a scene
//! /look 3d theme    the theme's own scene (the default)
//! ```
//!
//! The saved setting is a scene's name, `off`, or empty for "the theme's":
//! the prism theme brings its own scene (`crew_theme::ThemeId::scene`) and
//! every other theme none, so the default changed nothing for anyone who had
//! not chosen a scene, and `/theme prism` is a 3D theme without a second
//! command.
//!
//! Its clock is its own, not the gradient's wash: the wash turns only on the
//! themes with a drift period, and the scene should move on every theme.
//! Like the wash it turns only while a pane works, on frames that work was
//! already drawing, and holds at Motion off, so an idle window costs nothing.
use crew_render::stage3d::{self, Stage};

use crate::app::CrewApp;

/// Ms per turn of the scene while a pane works: slow enough to be a mood.
pub(crate) const TURN_MS: u64 = 40_000;

/// A saved setting as it is kept: a scene's name, `off`, or empty for the
/// theme's own.
pub(crate) fn clean(s: String) -> String {
    match s.trim() {
        "off" | "none" => "off".to_string(),
        t => Stage::parse(t).map_or(String::new(), |st| st.name().to_string()),
    }
}

/// The scene to draw for a saved setting on the live theme: a named scene
/// on every theme, none for `off`, else the theme's own (prism's, or none).
pub(crate) fn resolve(setting: &str, theme: crew_theme::ThemeId) -> Option<Stage> {
    match setting {
        "off" => None,
        "" => theme.scene().and_then(Stage::parse),
        name => Stage::parse(name),
    }
}

/// The picker's rows for `/look 3d `.
pub(crate) fn ladder() -> Vec<(String, String)> {
    let mut rows = vec![
        (
            "theme".to_string(),
            "the theme's own scene \u{2014} the default (prism has one)".to_string(),
        ),
        ("off".to_string(), "no scene on any theme".to_string()),
    ];
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
        Stage::Prism => "glass prisms drifting in a fog of the page",
    }
}

/// What `/look 3d` says it is set to.
fn report(setting: &str) -> String {
    let id = crew_theme::current_id();
    match (setting, resolve(setting, id)) {
        ("", Some(s)) => format!("{} (the theme's)", s.name()),
        ("", None) => "off (the theme has no scene)".to_string(),
        (_, s) => s.map_or("off", Stage::name).to_string(),
    }
}

impl CrewApp {
    /// `/look 3d [<scene>|off|theme]`. Persisted; the next frame draws it.
    pub(crate) fn backdrop_3d_command(&mut self, arg: &str) {
        let names: Vec<&str> = Stage::ALL.iter().map(|s| s.name()).collect();
        let usage = format!("/look 3d <{}|off|theme>", names.join("|"));
        let msg = match arg {
            "" => format!(
                "3D backdrop: {} \u{2014} {usage}",
                report(&self.config.backdrop_3d)
            ),
            "off" | "none" => {
                self.config.backdrop_3d = "off".to_string();
                "3D backdrop off".to_string()
            }
            "theme" | "default" => {
                self.config.backdrop_3d.clear();
                format!("3D backdrop: {}", report(""))
            }
            name => match Stage::parse(name) {
                Some(s) => {
                    self.config.backdrop_3d = s.name().to_string();
                    format!("3D backdrop: {}{}", s.name(), solid_note())
                }
                None => {
                    self.set_status(format!("usage: {usage}"));
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
        let s = resolve(&self.config.backdrop_3d, crew_theme::current_id());
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
