//! Deterministic per-`@project` colors: hash the tag name to a slot in a
//! pool derived from the theme's own 16-slot `ansi` palette, so a mixed
//! todo list reads by project at a glance. Nothing is stored — every pane,
//! restart and platform agrees because the color IS the (lowercased) name.
//! The theme supplies the slot's color, so switching themes recolors every
//! tag consistently instead of reshuffling which tag is which.
use crate::{contrast_ratio, Theme};

/// Chromatic slots of the terminal palette — everything except the four
/// gray poles (0 black, 7 white, 8 bright black, 15 bright white). Slot
/// count is a constant, so a tag's slot is identical on every theme.
const CHROMATIC: [usize; 12] = [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14];

/// Pool entries are lifted (never dropped) to this WCAG floor against the
/// page — the crew-term fg/bg answer floor, reused shape and number.
const FLOOR: f32 = 3.0;

/// SplitMix-style fold of the lowercased name (the charrain hash shape —
/// deterministic stand-in for RNG; `DefaultHasher` is seed-random on some
/// platforms and the color must survive restarts).
fn fold(name: &str) -> u64 {
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    for ch in name.chars().flat_map(char::to_lowercase) {
        x = (x ^ ch as u64).wrapping_mul(0xFF51_AFD7_ED55_8CCD);
        x ^= x >> 33;
    }
    // Full fmix64 avalanche: the per-char fold alone leaves short names
    // clustered mod 12 (crew and home collided without it).
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    x ^ (x >> 33)
}

/// The pool slot a tag maps to — theme-independent, case-insensitive.
pub fn tag_slot(name: &str) -> usize {
    (fold(name) % CHROMATIC.len() as u64) as usize
}

/// The color a `@name` tag renders in on theme `t`: the hashed slot's ansi
/// color, contrast-lifted against the page. Total for any non-empty name
/// (and harmlessly defined on the empty string too).
pub fn tag_color(name: &str, t: &Theme) -> (u8, u8, u8) {
    lift(t.ansi[CHROMATIC[tag_slot(name)]], t)
}

/// Blend `c` toward the theme's ink in tenths until it clears [`FLOOR`]
/// against the page — lifted, not dropped, so the pool size (and thus every
/// tag's slot) never varies by theme. Ink itself is the k=10 endpoint and
/// every theme's ink clears the floor by a wide margin.
fn lift(c: (u8, u8, u8), t: &Theme) -> (u8, u8, u8) {
    let mix = |a: u8, b: u8, k: u32| ((a as u32 * (10 - k) + b as u32 * k) / 10) as u8;
    for k in 0..=10 {
        let cand = (
            mix(c.0, t.ink.0, k),
            mix(c.1, t.ink.1, k),
            mix(c.2, t.ink.2, k),
        );
        if contrast_ratio(cand, t.page_bg) >= FLOOR {
            return cand;
        }
    }
    t.ink
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ALL_THEMES;

    /// Pinned name → slot table: the hash is a public contract (colors must
    /// survive restarts and upgrades), so the expected values are literals.
    /// Catches "always slot 0" mutants and accidental hash changes alike.
    #[test]
    fn slots_are_stable_and_case_insensitive() {
        for (name, want) in [("crew", 4), ("home", 7), ("work", 8), ("errands", 0)] {
            assert_eq!(tag_slot(name), want, "slot moved for @{name}");
        }
        assert_eq!(tag_slot("Crew"), tag_slot("crew"), "case must not matter");
        assert_eq!(tag_slot("HOME"), tag_slot("home"), "case must not matter");
        assert_ne!(
            tag_slot("crew"),
            tag_slot("home"),
            "distinct tags should spread"
        );
    }

    #[test]
    fn exotic_names_map_in_range_without_panicking() {
        for name in ["日本語", "🎉party", "a", "ß", ""] {
            assert!(tag_slot(name) < CHROMATIC.len(), "@{name} out of range");
        }
    }

    /// Same tag, same slot, on every theme — only the palette varies.
    #[test]
    fn a_tags_color_comes_from_the_same_slot_on_every_theme() {
        let slot = tag_slot("crew");
        for id in ALL_THEMES {
            let t = id.theme();
            assert_eq!(
                tag_color("crew", t),
                lift(t.ansi[CHROMATIC[slot]], t),
                "slot drifted on {}",
                id.as_str()
            );
        }
    }

    /// The arbiter: every pool entry, on every theme, clears the floor —
    /// concrete ratios in the failure message, not a boolean.
    #[test]
    fn every_pool_entry_clears_the_contrast_floor_on_every_theme() {
        for id in ALL_THEMES {
            let t = id.theme();
            for &slot in &CHROMATIC {
                let lifted = lift(t.ansi[slot], t);
                let ratio = contrast_ratio(lifted, t.page_bg);
                assert!(
                    ratio >= FLOOR,
                    "{} ansi[{slot}] lifted to {lifted:?} is {ratio:.2} vs page {:?} (< {FLOOR})",
                    id.as_str(),
                    t.page_bg
                );
            }
        }
    }

    /// Lifting must preserve an already-passing color untouched.
    #[test]
    fn lift_is_identity_when_the_color_already_passes() {
        let t = crate::PAPER_DARK;
        let ink = t.ink;
        assert_eq!(lift(ink, &t), ink);
    }
}
