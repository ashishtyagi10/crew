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
#[path = "tagcolor_tests.rs"]
mod tests;
