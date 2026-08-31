//! What `/theme` says about the CURRENT selection.
//!
//! A rotation mode names itself ("theme: light") and that is the whole story
//! — but `auto` has moving parts (which clock is deciding, the side it is
//! serving, the side it is not) and used to report only its own name. A
//! pairing configured for the appearance you are not in then has no symptom
//! whatsoever: a `theme_light` line under a dark OS renders exactly like a
//! config line that was ignored, so the honest reading of "I set that theme
//! and nothing happened" is that crew never said which half of `auto` was
//! live. This module says it.
//!
//! Since `auto` gained its clock fallback there is a second silent state to
//! name: a PINNED OS appearance means the light-hours window is deciding, not
//! the OS. Without saying so, "auto is dark at noon" and "auto is dark because
//! you told the OS to be dark" are the same sentence.
use crew_theme::{RandomMode, Selection};

/// `HH:MM` from minutes past midnight, for echoing the window back.
fn hhmm(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Build the report from explicit state — pure, so every branch is testable
/// without touching the process-global theme.
fn report(
    label: &str,
    mode: Option<RandomMode>,
    os_dark: bool,
    os_auto: bool,
    daylight: bool,
    hours: (u16, u16),
    pools: (Option<Selection>, Option<Selection>),
) -> String {
    if mode != Some(RandomMode::Auto) {
        return format!("theme: {label}");
    }
    let (dark, light) = pools;
    // An unpaired side is the built-in paper pool for that appearance, which
    // is exactly what the `dark`/`light` modes are — so naming them is both
    // accurate and something the user can type back at `/theme`.
    let dark = dark.map_or("dark", Selection::label);
    let light = light.map_or("light", Selection::label);
    // The same two clocks `crew_theme::auto_dark` weighs, in the same order.
    let now_dark = if os_auto { os_dark } else { !daylight };
    let (now, live, other_name, other) = if now_dark {
        ("dark", dark, "light", light)
    } else {
        ("light", light, "dark", dark)
    };
    if os_auto {
        format!(
            "theme: auto \u{2014} OS is {now}, serving {live}; the {other_name} half is {other} \
             (it shows when the OS turns {other_name})"
        )
    } else {
        let (from, to) = hours;
        let clock = if daylight { "day" } else { "night" };
        format!(
            "theme: auto \u{2014} the OS appearance is pinned, so the clock decides: it is \
             {clock}, serving {live}; the {other_name} half is {other} (light hours are \
             {}\u{2013}{}, set with auto_light_from / auto_light_to)",
            hhmm(from),
            hhmm(to)
        )
    }
}

/// The report for the live theme state.
pub(crate) fn live_report() -> String {
    report(
        crew_theme::selection_label(),
        crew_theme::mode(),
        crew_theme::os_dark(),
        crew_theme::os_auto(),
        crew_theme::daylight(),
        crew_theme::light_hours(),
        crew_theme::auto_pools(),
    )
}

#[cfg(test)]
#[path = "themereport_tests.rs"]
mod tests;
