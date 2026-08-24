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
///
/// On a phosphor tube the ansi slots are no basis for this at all — see
/// [`tube_rung`].
pub fn tag_color(name: &str, t: &Theme) -> (u8, u8, u8) {
    slot_color(tag_slot(name), t)
}

/// The colour of pool `slot` on theme `t` — the whole pool is enumerable, so
/// "no two tags look the same" is a property the suite can check directly
/// rather than by hashing names until every slot has been seen.
pub fn slot_color(slot: usize, t: &Theme) -> (u8, u8, u8) {
    let slot = slot % CHROMATIC.len();
    // A phosphor tube is not a set of hues; borrowing the ansi slots there
    // collapses the pool. See `tube_rung`. (`crt.is_some()` alone would catch
    // the modern family too, whose bloom-only style is not a tube.)
    if t.is_tube() {
        return tube_rung(slot, t);
    }
    lift(t.ansi[CHROMATIC[slot]], t)
}

/// The tag pool on a phosphor tube: an evenly spaced BRIGHTNESS ladder in the
/// tube's one hue, rather than twelve borrowed ansi slots.
///
/// Borrowing was wrong here in a way only measurement shows. On a coloured
/// page the twelve chromatic slots are twelve hues and the closest pair sits
/// Δ 0.057 apart — two visible steps, tellable. On a tube every slot is the
/// SAME hue (that is what a tube is), so the pool separates by brightness
/// alone — and the ansi slots were derived for shell output, where everything
/// is bright. All twelve landed inside L 0.62–0.95, which put the closest pair
/// at **Δ 0.017**, below the Δ 0.027 that is one visible step. Two different
/// `@projects` rendered as the same colour.
///
/// The room was there and unused: the page is near-black, so the legible range
/// runs from wherever [`FLOOR`] is met all the way to the top of the phosphor,
/// roughly twice what the slots were using. Spreading the twelve rungs evenly
/// over it is the same answer the ansi work reached for the tubes — separate
/// by brightness, because hue is not available — applied to the one pool that
/// never got it.
fn tube_rung(slot: usize, t: &Theme) -> (u8, u8, u8) {
    let hue_from = crate::oklch::from_srgb(t.ansi[CHROMATIC[slot]]);
    // The phosphor's own hue and saturation, read off the brightest chromatic
    // slot so the ladder is as vivid as the tube gets rather than as vivid as
    // whichever slot the tag happened to hash to.
    let peak = CHROMATIC
        .iter()
        .map(|&i| crate::oklch::from_srgb(t.ansi[i]))
        .fold(hue_from, |a, b| if b.c > a.c { b } else { a });
    let dim = crate::oklch::solve_for_contrast(
        t.page_bg,
        peak.h,
        peak.c,
        FLOOR,
        crate::oklch::Toward::for_page(t.page_bg),
    );
    let lo = crate::oklch::from_srgb(dim).l;
    // The top of the ladder is the brightest the tube's own palette goes —
    // read off the chromatic slots rather than `ink`, which on `crt-green`
    // sits at L 0.87 while the slots reach 0.95. Every 0.01 of range here is
    // spread across eleven gaps, so it is not a detail.
    let hi = CHROMATIC
        .iter()
        .map(|&i| crate::oklch::from_srgb(t.ansi[i]).l)
        .fold(crate::oklch::from_srgb(t.ink).l, f32::max)
        .max(lo + 0.05);
    let n = CHROMATIC.len() as f32;
    let l = lo + (hi - lo) * (slot as f32 / (n - 1.0));
    crate::oklch::Oklch::new(l, peak.c, peak.h).to_srgb()
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

    /// Same tag, same slot, on every theme — only the palette varies. That
    /// is the invariant the whole scheme rests on: switching themes recolours
    /// every tag, but never reshuffles which tag is which.
    #[test]
    fn a_tags_slot_is_the_same_on_every_theme() {
        for name in ["crew", "home", "work", "errands", "日本語"] {
            let slot = tag_slot(name);
            for id in ALL_THEMES {
                let t = id.theme();
                assert_eq!(
                    tag_color(name, t),
                    slot_color(slot, t),
                    "@{name} took a different slot on {}",
                    id.as_str()
                );
            }
        }
    }

    /// The pool entry a slot maps to, on every palette.
    fn pool(t: &Theme) -> Vec<(u8, u8, u8)> {
        (0..CHROMATIC.len()).map(|s| slot_color(s, t)).collect()
    }

    /// No two `@projects` may render in colours a person cannot tell apart.
    ///
    /// The defect this floor was written for: on the tubes the pool used to
    /// borrow twelve ansi slots that are all the SAME hue and all bright,
    /// landing every entry inside L 0.62–0.95 and putting the closest pair
    /// Δ 0.017 apart — below the Δ 0.027 that is one visible step. Two
    /// different projects were the same colour.
    #[test]
    fn no_two_tags_render_the_same_colour() {
        const FLOOR_D: f32 = 0.035;
        for id in ALL_THEMES {
            let t = id.theme();
            let p = pool(t);
            for a in 0..p.len() {
                for b in a + 1..p.len() {
                    let d = crate::oklch::distance(p[a], p[b]);
                    assert!(
                        d >= FLOOR_D,
                        "{}: tag slots {a} {:?} and {b} {:?} are Δ {d:.4} apart \
                         (floor {FLOOR_D}) — two projects, one colour",
                        id.as_str(),
                        p[a],
                        p[b]
                    );
                }
            }
        }
        // Live, not vacuous: the tubes sit just above it by construction, and
        // that is the whole point — twelve rungs on one hue is as far as a
        // monochrome page stretches.
        let tightest = ALL_THEMES
            .into_iter()
            .flat_map(|id| {
                let p = pool(id.theme());
                (0..p.len())
                    .flat_map(move |a| {
                        let p = p.clone();
                        (a + 1..p.len()).map(move |b| crate::oklch::distance(p[a], p[b]))
                    })
                    .collect::<Vec<_>>()
            })
            .fold(f32::MAX, f32::min);
        assert!(
            tightest < FLOOR_D + 0.01,
            "the closest pair anywhere is Δ {tightest:.4} against a floor of \
             {FLOOR_D} — the floor has stopped constraining the pools"
        );
    }

    /// The arbiter: every pool entry, on every theme, clears the floor —
    /// concrete ratios in the failure message, not a boolean. Measured through
    /// `slot_color`, so the tube ladder is held to it too (its dim rung is
    /// solved to exactly this number, which makes this the tightest check in
    /// the file).
    #[test]
    fn every_pool_entry_clears_the_contrast_floor_on_every_theme() {
        for id in ALL_THEMES {
            let t = id.theme();
            for slot in 0..CHROMATIC.len() {
                let c = slot_color(slot, t);
                let ratio = contrast_ratio(c, t.page_bg);
                assert!(
                    ratio >= FLOOR - 0.02,
                    "{} slot {slot} is {c:?} at {ratio:.2} vs page {:?} (< {FLOOR})",
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
