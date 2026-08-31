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
#[path = "restartnote_tests.rs"]
mod tests;
