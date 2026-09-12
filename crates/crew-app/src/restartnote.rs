//! The parked-update reminder: the stats-card legend naming the version a
//! background install already put on disk, and the blink clock the RESTART
//! card (see [`crate::restartcard`]) runs on. Pure helpers — the state lives
//! on `CrewApp.parked_update`, the painting in `navcard`.

/// `crew v<current> \u{2192} v<new>` when it fits in `max_cols` title columns;
/// otherwise the compact `\u{2192} v<new>`, which keeps the half that is news.
/// Even the compact form is prefix-truncated by `titled_card` if it still
/// overflows — acceptable, the new version leads the string.
///
/// There is no `/update` call-to-action any more: the update has already
/// installed itself, and the thing left to do is a button, not a command.
pub(crate) fn legend(new_version: &str, max_cols: usize) -> String {
    let full = format!(
        concat!("crew v", env!("CARGO_PKG_VERSION"), " \u{2192} v{}"),
        new_version
    );
    if full.chars().count() <= max_cols {
        return full;
    }
    format!("\u{2192} v{}", new_version)
}

/// Whether the restart button is currently blinking — true whenever an
/// install is parked, unless motion is off, in which case it is drawn
/// steady and costs no frames.
pub(crate) fn animating() -> bool {
    crate::motion::level() != crate::motion::MotionLevel::Off
}

#[cfg(test)]
#[path = "restartnote_tests.rs"]
mod tests;
