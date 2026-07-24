//! The parked-update restart reminder: legend text and blink styling for
//! the nav stats card while a newer installed binary waits for /restart.
//! Pure helpers — state lives on CrewApp.parked_update, painting in navcard.

/// `crew v<current> → v<new> · /restart`
pub(crate) fn legend(new_version: &str) -> String {
    format!(
        concat!(
            "crew v",
            env!("CARGO_PKG_VERSION"),
            " \u{2192} v{} \u{b7} /restart"
        ),
        new_version
    )
}

/// Accent↔dim alternation on the attention clock for the first PULSE_MS,
/// then steady accent — same cost model as pane attention markers.
pub(crate) fn legend_fg(now_ms: u64, parked_at_ms: u64) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    if animating(now_ms, parked_at_ms)
        && ((now_ms - parked_at_ms) / crate::attention::BLINK_MS) % 2 == 1
    {
        return t.legend_off;
    }
    crate::palette::accent()
}

/// True only inside the blink window — the only time redraws are driven.
pub(crate) fn animating(now_ms: u64, parked_at_ms: u64) -> bool {
    now_ms.saturating_sub(parked_at_ms) < crate::attention::PULSE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_names_both_versions_and_the_restart_command() {
        let s = legend("9.9.9");
        assert!(
            s.starts_with(concat!("crew v", env!("CARGO_PKG_VERSION"))),
            "{s}"
        );
        assert!(s.contains("\u{2192} v9.9.9"), "{s}"); // →
        assert!(s.ends_with("\u{b7} /restart"), "{s}"); // ·
    }

    #[test]
    fn legend_blinks_through_the_pulse_window_then_holds_accent() {
        let _g = crate::palette::test_guard();
        let accent = crate::palette::accent();
        let t0 = 10_000u64;
        // Inside the pulse window the fg alternates on the BLINK_MS half-period.
        let a = legend_fg(t0, t0);
        let b = legend_fg(t0 + crate::attention::BLINK_MS, t0);
        assert_ne!(a, b, "must alternate each half-period");
        assert!(a == accent || b == accent, "one phase is the accent");
        // After the pulse window: steady accent, and no more redraw driving.
        let late = t0 + crate::attention::PULSE_MS + 1;
        assert_eq!(legend_fg(late, t0), accent);
        assert!(animating(t0 + 1, t0));
        assert!(!animating(late, t0));
    }
}
