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
mod tests {
    use super::*;
    use crew_theme::ThemeId;

    const HOURS: (u16, u16) = (7 * 60, 19 * 60);

    /// The OS-following case: the clock arguments must not matter.
    fn os_report(
        label: &str,
        mode: Option<RandomMode>,
        os_dark: bool,
        pools: (Option<Selection>, Option<Selection>),
    ) -> String {
        let msg = report(label, mode, os_dark, true, false, HOURS, pools);
        assert_eq!(
            msg,
            report(label, mode, os_dark, true, true, HOURS, pools),
            "daylight must not reach the report while the OS is self-switching"
        );
        msg
    }

    #[test]
    fn a_plain_mode_reports_only_itself() {
        assert_eq!(
            os_report("light", Some(RandomMode::Light), true, (None, None)),
            "theme: light"
        );
        assert_eq!(
            os_report("blossom", None, false, (None, None)),
            "theme: blossom"
        );
    }

    #[test]
    fn auto_names_the_live_half_and_the_dormant_one() {
        // The shape this module exists for: a light half pinned to something
        // while macOS is dark. The old report was the bare word "auto", which
        // is why such a pairing read as a no-op.
        let msg = os_report(
            "auto",
            Some(RandomMode::Auto),
            true,
            (None, Some(Selection::Fixed(ThemeId::Blossom))),
        );
        assert!(msg.contains("OS is dark, serving dark"), "{msg}");
        assert!(msg.contains("the light half is blossom"), "{msg}");
        assert!(msg.contains("turns light"), "{msg}");

        // Flip the appearance and the same pairing is the one on screen.
        let msg = os_report(
            "auto",
            Some(RandomMode::Auto),
            false,
            (None, Some(Selection::Fixed(ThemeId::Blossom))),
        );
        assert!(msg.contains("OS is light, serving blossom"), "{msg}");
        assert!(msg.contains("the dark half is dark"), "{msg}");
    }

    #[test]
    fn an_unpaired_side_reports_the_pool_it_actually_serves() {
        // `None` is not "nothing" — it is the built-in paper pool, whose name
        // is `dark`/`light`; a pinned palette names itself.
        let msg = os_report(
            "auto",
            Some(RandomMode::Auto),
            false,
            (Some(Selection::Fixed(ThemeId::CrtGreen)), None),
        );
        assert!(msg.contains("serving light"), "{msg}");
        assert!(msg.contains("the dark half is crt-green"), "{msg}");
    }

    #[test]
    fn a_pinned_os_reports_the_clock_and_ignores_the_os_appearance() {
        // The reported bug: macOS pinned to Dark, broad daylight. `auto` must
        // serve the light half AND say why, or the fix is indistinguishable
        // from the bug.
        let msg = report(
            "auto",
            Some(RandomMode::Auto),
            true,
            false,
            true,
            HOURS,
            (None, None),
        );
        assert!(msg.contains("the OS appearance is pinned"), "{msg}");
        assert!(msg.contains("it is day, serving light"), "{msg}");
        assert!(msg.contains("the dark half is dark"), "{msg}");
        assert!(msg.contains("07:00\u{2013}19:00"), "{msg}");
        assert!(!msg.contains("OS is dark"), "{msg}");

        // Same pinned-dark OS after hours: now the clock agrees with it, and
        // the report says the clock said so.
        let night = report(
            "auto",
            Some(RandomMode::Auto),
            true,
            false,
            false,
            HOURS,
            (None, None),
        );
        assert!(night.contains("it is night, serving dark"), "{night}");
        assert!(night.contains("the light half is light"), "{night}");
    }

    #[test]
    fn a_pinned_os_echoes_the_configured_window_not_the_default() {
        let msg = report(
            "auto",
            Some(RandomMode::Auto),
            false,
            false,
            false,
            (5 * 60 + 30, 21 * 60 + 5),
            (Some(Selection::Fixed(ThemeId::CrtGreen)), None),
        );
        assert!(msg.contains("05:30\u{2013}21:05"), "{msg}");
        // Pinned LIGHT at night still goes dark: once the OS stops changing,
        // the clock is the only thing left that can.
        assert!(msg.contains("it is night, serving crt-green"), "{msg}");
    }

    #[test]
    fn hhmm_pads_both_halves() {
        assert_eq!(hhmm(0), "00:00");
        assert_eq!(hhmm(7 * 60), "07:00");
        assert_eq!(hhmm(23 * 60 + 59), "23:59");
    }
}
