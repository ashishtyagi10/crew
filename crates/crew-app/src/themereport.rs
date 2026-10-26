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
    let looks: Vec<&str> = crew_theme::auto_looks().map(RandomMode::as_str).collect();
    format!(
        "theme: auto \u{2014} {}, a new look every {} minutes; now {} ({}), next {}",
        looks.join(" \u{2192} "),
        crew_theme::ROTATE_MS / 60_000,
        current.mode().as_str(),
        current.as_str(),
        crew_theme::auto_next(current).as_str(),
    )
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
