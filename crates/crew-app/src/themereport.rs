//! What `/theme` says about the CURRENT selection.
//!
//! A rotation mode names itself ("theme: light") and that is the whole story
//! — but `auto` has moving parts (the side it is serving, the side it is
//! not) and used to report only its own name. A
//! pairing configured for the appearance you are not in then has no symptom
//! whatsoever: a `theme_light` line under a dark OS renders exactly like a
//! config line that was ignored, so the honest reading of "I set that theme
//! and nothing happened" is that crew never said which half of `auto` was
//! live. This module says it.
use crew_theme::{RandomMode, Selection};

/// Build the report from explicit state — pure, so every branch is testable
/// without touching the process-global theme.
fn report(
    label: &str,
    mode: Option<RandomMode>,
    os_dark: bool,
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
    let (now, live, other_name, other) = if os_dark {
        ("dark", dark, "light", light)
    } else {
        ("light", light, "dark", dark)
    };
    format!(
        "theme: auto \u{2014} OS is {now}, serving {live}; the {other_name} half is {other} \
         (it shows when the OS turns {other_name})"
    )
}

/// The report for the live theme state.
pub(crate) fn live_report() -> String {
    report(
        crew_theme::selection_label(),
        crew_theme::mode(),
        crew_theme::auto_dark(),
        crew_theme::auto_pools(),
    )
}

#[cfg(test)]
#[path = "themereport_tests.rs"]
mod tests;
