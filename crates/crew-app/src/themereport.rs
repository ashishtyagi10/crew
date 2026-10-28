//! What `/theme` says about the CURRENT selection.
//!
//! A rotation mode names itself ("theme: light") and that is the whole story
//! — but `auto` walks every look in turn, so its name alone says neither
//! what is on screen nor what comes next. This module says both.
use crew_theme::{RandomMode, ThemeId};

/// Build the report from explicit state — pure, so every branch is testable
/// without touching the process-global theme.
fn report(label: &str, mode: Option<RandomMode>, current: ThemeId) -> String {
    if mode != Some(RandomMode::Auto) {
        return format!("theme: {label}");
    }
    let looks: Vec<String> = crew_theme::auto_looks().map(look_name).collect();
    format!(
        "theme: auto \u{2014} {}, a new one every {} minutes; now {}, next {}",
        looks.join(" \u{2192} "),
        crew_theme::ROTATE_MS / 60_000,
        on_screen(current),
        coming(current),
    )
}

/// A look as the walk lists it: `classic (all 5)` for one `auto` shows
/// palette by palette.
fn look_name(look: RandomMode) -> String {
    match crew_theme::auto_steps(look) {
        1 => look.as_str().to_string(),
        n => format!("{} (all {n})", look.as_str()),
    }
}

/// The look on screen and its palette: `glass (nebula)`.
fn on_screen(current: ThemeId) -> String {
    format!("{} ({})", current.mode().as_str(), current.as_str())
}

/// What comes after `current`: the look, and its palette when that is
/// already decided and the look has more than one (`classic (monokai)`).
fn coming(current: ThemeId) -> String {
    let look = crew_theme::auto_next(current);
    match crew_theme::auto_next_palette(current) {
        Some(id) if look.sole_palette().is_none() => on_screen(id),
        _ => look.as_str().to_string(),
    }
}

/// The report for the live theme state.
pub(crate) fn live_report() -> String {
    report(
        crew_theme::selection_label(),
        crew_theme::mode(),
        crew_theme::current_id(),
    )
}

#[cfg(test)]
#[path = "themereport_tests.rs"]
mod tests;
