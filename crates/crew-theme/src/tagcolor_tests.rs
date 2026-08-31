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
