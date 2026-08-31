//! `/font random`: rotate the UI font through the installed monospace
//! families on the shared 10-minute rotation clock (`crew_theme::ROTATE_MS`).
//! The rotated family lives HERE (`current`), never in `config.font_family`
//! — unrelated `config.save()` calls must not persist a rotated pick, and a
//! restart returns to the user's pinned family.

/// Rotation state on the app. `pool` is scanned once per session (loading
/// faces is not free) and cached; `None` = not scanned yet.
#[derive(Default)]
pub(crate) struct FontRotate {
    pub on: bool,
    pub last_ms: u64,
    pub pool: Option<Vec<String>>,
    pub current: Option<String>,
    /// The theme whose font was last applied. `poll` compares the live theme
    /// against this to notice a change ONCE, whatever caused it (`/theme`, the
    /// picker, `Ctrl+Shift+L`, or the rotation tick) — four call sites would
    /// otherwise each need to remember to apply the theme's font.
    pub themed: Option<crew_theme::ThemeId>,
}

impl FontRotate {
    /// Whether a rotation is due at `now_ms` (only while on).
    pub(crate) fn due(&self, now_ms: u64) -> bool {
        self.on && now_ms.saturating_sub(self.last_ms) >= crew_theme::ROTATE_MS
    }
}

/// A family from `pool` that isn't `current`, deterministically from `seed`
/// (same hash recipe as `crew_theme::random_pick`). `None` when the pool has
/// no alternative.
pub(crate) fn pick(pool: &[String], current: Option<&str>, seed: u64) -> Option<String> {
    let others: Vec<&String> = pool
        .iter()
        .filter(|f| Some(f.as_str()) != current)
        .collect();
    if others.is_empty() {
        return None;
    }
    let idx = (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % others.len();
    Some(others[idx].clone())
}

#[cfg(test)]
#[path = "fontrotate_tests.rs"]
mod tests;
