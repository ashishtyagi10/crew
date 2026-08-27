//! The parked-update reminder: legend text and blink styling for the nav
//! stats card while a newer installed binary waits for the `/update` that
//! restarts into it. Pure helpers — state lives on CrewApp.parked_update,
//! painting in navcard.

/// `crew v<current> → v<new> · /update` when it fits in `max_cols` title
/// columns; otherwise the compact `→ v<new> · /update` form that keeps the
/// actionable half (the new version and the `/update` call-to-action) —
/// the narrow nav sidebar rarely has room for the full form. Even the
/// compact form is prefix-truncated by `titled_card` if it still overflows
/// (acceptable — the version and `/update` lead the string).
pub(crate) fn legend(new_version: &str, max_cols: usize) -> String {
    let full = format!(
        concat!(
            "crew v",
            env!("CARGO_PKG_VERSION"),
            " \u{2192} v{} \u{b7} /update"
        ),
        new_version
    );
    if full.chars().count() <= max_cols {
        return full;
    }
    format!("\u{2192} v{} \u{b7} /update", new_version)
}

/// Accent↔dim alternation on the attention clock for the first PULSE_MS,
/// then steady accent — same cost model as pane attention markers.
pub(crate) fn legend_fg(now_ms: u64, parked_at_ms: u64) -> (u8, u8, u8) {
    let t = crew_theme::theme();
    let dt = now_ms.saturating_sub(parked_at_ms);
    if dt < crate::attention::PULSE_MS && (dt / crate::attention::BLINK_MS) % 2 == 1 {
        return t.legend_off;
    }
    crate::palette::accent()
}

/// True only inside the blink window (the elapsed time since `parked_at_ms`
/// is under `PULSE_MS`) — never panics or wraps if `now_ms` precedes
/// `parked_at_ms`, since the elapsed time is computed with `saturating_sub`.
pub(crate) fn animating(now_ms: u64, parked_at_ms: u64) -> bool {
    now_ms.saturating_sub(parked_at_ms) < crate::attention::PULSE_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legend_names_both_versions_and_the_update_command_at_generous_width() {
        let s = legend("9.9.9", 100);
        assert!(
            s.starts_with(concat!("crew v", env!("CARGO_PKG_VERSION"))),
            "{s}"
        );
        assert!(s.contains("\u{2192} v9.9.9"), "{s}"); // →
        assert!(s.ends_with("\u{b7} /update"), "{s}"); // ·
    }

    #[test]
    fn legend_falls_back_to_the_compact_form_when_narrow() {
        let s = legend("9.9.9", 25);
        assert!(
            !s.starts_with("crew v"),
            "narrow width must drop the current-version prefix: {s}"
        );
        assert!(s.contains("v9.9.9"), "{s}");
        assert!(s.contains("\u{b7} /update"), "{s}"); // ·
        assert!(s.ends_with("/update"), "{s}");
        assert!(s.chars().count() <= 25, "{s}");
    }

    #[test]
    fn legend_blinks_through_the_pulse_window_then_holds_accent() {
        let _g = crate::app::theme_test_guard();
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

    #[test]
    fn legend_fg_does_not_panic_when_now_precedes_parked_at() {
        let _g = crate::app::theme_test_guard();
        let _g = crate::palette::test_guard();
        let accent = crate::palette::accent();
        let parked_at = 10_000u64;
        // now < parked_at can't happen with the shared anim clock in practice,
        // but the saturating elapsed-time math must not panic/wrap on it —
        // it should read as "just parked" (phase 0, accent).
        assert_eq!(legend_fg(parked_at.saturating_sub(1), parked_at), accent);
    }
}
