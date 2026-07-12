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
mod tests {
    use super::*;

    fn pool() -> Vec<String> {
        vec!["Menlo".into(), "Monaco".into(), "Hack".into()]
    }

    #[test]
    fn pick_never_returns_the_current_family() {
        for seed in 0..50u64 {
            let p = pick(&pool(), Some("Menlo"), seed).unwrap();
            assert_ne!(p, "Menlo", "seed {seed}");
        }
    }

    #[test]
    fn pick_is_deterministic_for_a_seed() {
        assert_eq!(
            pick(&pool(), Some("Menlo"), 7),
            pick(&pool(), Some("Menlo"), 7)
        );
    }

    #[test]
    fn pick_returns_none_when_no_alternative_exists() {
        assert_eq!(pick(&["Menlo".to_string()], Some("Menlo"), 1), None);
        assert_eq!(pick(&[], None, 1), None);
    }

    #[test]
    fn due_gates_on_the_shared_rotate_clock() {
        let mut r = FontRotate {
            on: true,
            last_ms: 1_000,
            ..Default::default()
        };
        assert!(!r.due(1_000 + crew_theme::ROTATE_MS - 1));
        assert!(r.due(1_000 + crew_theme::ROTATE_MS));
        r.on = false;
        assert!(!r.due(1_000 + crew_theme::ROTATE_MS));
    }
}
