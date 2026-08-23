//! Is it daytime? — the clock half of the `auto` theme.
//!
//! `auto` follows the OS appearance, which is the right answer while the OS
//! switches itself. It is the WRONG answer when the appearance is pinned:
//! a Mac fixed to Dark never turns light, so `auto` silently degrades into
//! `dark` and the mode's own promise ("light by day, dark by night") never
//! comes true. When [`crate::osappearance`] reports a pinned appearance,
//! crew consults this module instead.
//!
//! The window is wall-clock, not solar: crew has no location and asking for
//! one (CoreLocation, an IP lookup) costs a permission prompt or a network
//! call to decide a colour. `auto_light_from` / `auto_light_to` let anyone
//! who cares dial their own sunrise.

/// Minutes past local midnight, parsed from `HH:MM`. Returns `None` on
/// anything that is not a real time of day, so a typo falls back to the
/// default window rather than resolving to midnight (a bad string that
/// silently means 00:00 would pin `auto` to dark all day — the exact class
/// of invisible failure this parse exists to avoid).
pub(crate) fn parse_hhmm(s: &str) -> Option<u16> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u16 = h.trim().parse().ok()?;
    let m: u16 = m.trim().parse().ok()?;
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// The default light-hours window: 07:00 to 19:00.
pub(crate) const DEFAULT_FROM: u16 = 7 * 60;
pub(crate) const DEFAULT_TO: u16 = 19 * 60;

/// Whether `now` (minutes past midnight) falls inside `[from, to)`.
///
/// A window that WRAPS past midnight (`to <= from`, e.g. 20:00 → 06:00) is
/// read as "daylight spans midnight" rather than rejected — someone whose day
/// really does start at 20:00 gets what they asked for. `from == to` is the
/// degenerate case and means no daylight at all (always dark), which is at
/// least a state the user can see and undo.
pub(crate) fn is_day(now: u16, from: u16, to: u16) -> bool {
    if from == to {
        false
    } else if from < to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

/// `is_day` against the local wall clock.
pub(crate) fn is_day_now(from: u16, to: u16) -> bool {
    use chrono::Timelike;
    let t = chrono::Local::now();
    let now = (t.hour() * 60 + t.minute()) as u16;
    is_day(now, from, to)
}

#[cfg(test)]
#[path = "daylight_tests.rs"]
mod tests;
