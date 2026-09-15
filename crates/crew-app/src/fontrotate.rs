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

/// A family from `pool` to rotate to, deterministically from `seed` (same
/// hash recipe as `crew_theme::random_pick`). `None` when the pool holds no
/// other TYPEFACE.
///
/// Two things happen here that a flat pick over family names cannot do.
///
/// It rotates between typefaces, not spellings. A machine with
/// `JetBrainsMono Nerd Font` and `JetBrainsMono Nerd Font Mono` installed
/// carries one face under two names, and a rotation that swapped one for the
/// other announced a new font and changed nothing on screen.
///
/// And the user's [`crew_theme::FAVORITES`] come up
/// [`crew_theme::FAVORITE_WEIGHT`] times as often as anything else — each
/// typeface gets that many tickets rather than one. Weighting names instead
/// would hand the prize to whichever face happened to be installed under the
/// most spellings, which is not a taste.
pub(crate) fn pick(pool: &[String], current: Option<&str>, seed: u64) -> Option<String> {
    let here = current.map(crew_theme::typeface_key);
    // One entry per typeface, keeping the spelling this machine is best off
    // asking for; the face on screen is out of the draw in every spelling.
    let mut faces: Vec<(String, String)> = Vec::new();
    for fam in pool {
        let key = crew_theme::typeface_key(fam);
        if Some(&key) == here.as_ref() {
            continue;
        }
        match faces.iter_mut().find(|(k, _)| *k == key) {
            Some((_, name)) => {
                if better_spelling(fam, name) {
                    *name = fam.clone();
                }
            }
            None => faces.push((key, fam.clone())),
        }
    }
    if faces.is_empty() {
        return None;
    }
    let tickets: Vec<&String> = faces
        .iter()
        .flat_map(|(_, name)| {
            let n = match crew_theme::is_favorite(name) {
                true => crew_theme::FAVORITE_WEIGHT,
                false => 1,
            };
            std::iter::repeat_n(name, n)
        })
        .collect();
    let idx = (seed.wrapping_mul(6364136223846793005).rotate_right(29) as usize) % tickets.len();
    Some(tickets[idx].clone())
}

/// Whether `cand` is a better name to ask for than `held`, for one typeface.
///
/// The icon-bearing builds first — a Nerd Font spelling is the same face plus
/// the glyphs crew's marks are drawn from — and the `Mono` build of those
/// first of all, since its icons are one cell wide and this is a cell grid.
fn better_spelling(cand: &str, held: &str) -> bool {
    rank(cand) > rank(held)
}

fn rank(family: &str) -> u8 {
    let f = family.to_ascii_lowercase();
    let nerd = f.contains("nerd font") || f.ends_with(" nf");
    match (nerd, f.ends_with("mono")) {
        (true, true) => 3,
        (true, false) => 2,
        (false, _) => 1,
    }
}

#[cfg(test)]
#[path = "fontrotate_tests.rs"]
mod tests;
