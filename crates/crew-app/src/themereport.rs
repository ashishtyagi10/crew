//! What `/theme` says about the CURRENT selection.
//!
//! A rotation mode names itself ("theme: light") and that is the whole story
//! — but `auto` has three moving parts (the OS appearance, the side it is
//! serving, the side it is not) and used to report only its own name. A
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
        crew_theme::os_dark(),
        crew_theme::auto_pools(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crew_theme::ThemeId;

    #[test]
    fn a_plain_mode_reports_only_itself() {
        assert_eq!(
            report("light", Some(RandomMode::Light), true, (None, None)),
            "theme: light"
        );
        assert_eq!(
            report("daybreak", None, false, (None, None)),
            "theme: daybreak"
        );
    }

    #[test]
    fn auto_names_the_live_half_and_the_dormant_one() {
        // The shape this module exists for: a light half pinned to something
        // while macOS is dark. The old report was the bare word "auto", which
        // is why such a pairing read as a no-op.
        let msg = report(
            "auto",
            Some(RandomMode::Auto),
            true,
            (None, Some(Selection::Fixed(ThemeId::Daybreak))),
        );
        assert!(msg.contains("OS is dark, serving dark"), "{msg}");
        assert!(msg.contains("the light half is daybreak"), "{msg}");
        assert!(msg.contains("turns light"), "{msg}");

        // Flip the appearance and the same pairing is the one on screen.
        let msg = report(
            "auto",
            Some(RandomMode::Auto),
            false,
            (None, Some(Selection::Fixed(ThemeId::Daybreak))),
        );
        assert!(msg.contains("OS is light, serving daybreak"), "{msg}");
        assert!(msg.contains("the dark half is dark"), "{msg}");
    }

    #[test]
    fn an_unpaired_side_reports_the_pool_it_actually_serves() {
        // `None` is not "nothing" — it is the built-in paper pool, whose name
        // is `dark`/`light`; a pinned palette names itself.
        let msg = report(
            "auto",
            Some(RandomMode::Auto),
            false,
            (Some(Selection::Fixed(ThemeId::CrtGreen)), None),
        );
        assert!(msg.contains("serving light"), "{msg}");
        assert!(msg.contains("the dark half is crt-green"), "{msg}");
    }
}
