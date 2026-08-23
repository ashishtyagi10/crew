//! The checks `contrast_thresholds` never had. Each one corresponds to a gap
//! the goal named, and each fails against the palettes as they shipped —
//! measured before the derivation was written, so these are pinning real bugs
//! rather than describing the fix.
use super::*;
use crate::oklch::distance;
use crate::{contrast_ratio, ALL_THEMES};

const NAMES: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright black",
    "bright red",
    "bright green",
    "bright yellow",
    "bright blue",
    "bright magenta",
    "bright cyan",
    "bright white",
];

/// Two colours closer than this are not reliably separable. Anchored on the
/// scale in `oklch`: eight codes of grey measure Δ 0.027 and read as a step,
/// so 0.04 is comfortably past "might be the same colour".
const SEPARABLE: f32 = 0.04;

/// Gap one: the blacks and whites were skipped outright, and slot 0 bottomed
/// out at 1.36:1 — ANSI black on a dark theme was very nearly the background.
#[test]
fn every_slot_including_the_blacks_and_whites_is_legible() {
    for id in ALL_THEMES {
        let t = id.theme();
        let slots = AnsiRamp::fitted(t).slots();
        for (i, c) in slots.iter().enumerate() {
            let cr = contrast_ratio(*c, t.term_bg);
            // 3.0 is the existing floor for the chromatic slots; the neutrals
            // were held to nothing at all.
            assert!(
                cr >= 3.0,
                "{}: ansi[{i}] ({}) = {cr:.2} against term_bg — below the floor",
                id.as_str(),
                NAMES[i]
            );
        }
    }
}

/// Gap two: nothing above the floor. Slots ranged 4.64..17.34 depending on
/// theme and slot, so the same colour meant different things in different
/// themes — the text ladder's problem, transposed onto shell output.
#[test]
fn the_chromatic_slots_share_one_level_within_a_theme() {
    for id in ALL_THEMES.iter().filter(|id| !id.is_crt()) {
        let t = id.theme();
        let slots = AnsiRamp::fitted(t).slots();
        let ratios: Vec<f32> = (1..7)
            .map(|i| contrast_ratio(slots[i], t.term_bg))
            .collect();
        let (lo, hi) = ratios
            .iter()
            .fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(
            hi / lo < 1.10,
            "{}: chromatic slots span {lo:.2}..{hi:.2} ({:.2}x) — one hue is \
             shouting louder than the others",
            id.as_str(),
            hi / lo
        );
    }
}

/// Gap three, and the one a user actually hits: nothing compared the slots to
/// each other. `crt-amber` shipped ANSI green `(240, 200, 40)` and yellow
/// `(255, 200, 30)` at Δ 0.0227 — closer than eight codes of grey. `ls
/// --color` and `git diff` draw with these.
///
/// What "distinguishable" means depends on how the palette separates things,
/// and conflating the two would demand something impossible. A **chromatic**
/// palette separates by hue, so all twelve chromatic slots must be mutually
/// distinct. A **monochrome** tube has one hue and twelve brightnesses: base
/// green and bright red necessarily land on adjacent rungs, and calling that a
/// defect would mean abolishing the phosphor. There, the row is the unit —
/// the six base slots must be distinct from one another, and so must the six
/// bright ones.
#[test]
fn no_two_slots_are_indistinguishable() {
    for id in ALL_THEMES {
        let t = id.theme();
        let slots = AnsiRamp::fitted(t).slots();
        let groups: Vec<Vec<usize>> = if id.is_crt() {
            vec![(1..7).collect(), (9..15).collect()]
        } else {
            vec![(1..7).chain(9..15).collect()]
        };
        for group in groups {
            for (n, &a) in group.iter().enumerate() {
                for &b in &group[n + 1..] {
                    let d = distance(slots[a], slots[b]);
                    assert!(
                        d >= SEPARABLE,
                        "{}: ansi[{a}] ({}) and ansi[{b}] ({}) are Δ {d:.4} \
                         apart — {:?} vs {:?} cannot be told apart",
                        id.as_str(),
                        NAMES[a],
                        NAMES[b],
                        slots[a],
                        slots[b]
                    );
                }
            }
        }
    }
}

/// A bright slot must actually be brighter than its base, by a consistent
/// amount. Shipped, the step ranged -0.002 to +0.075 across hues within a
/// single theme — some "bright" colours were darker than their base.
#[test]
fn every_bright_slot_is_a_consistent_step_from_its_base() {
    for id in ALL_THEMES {
        let t = id.theme();
        let slots = AnsiRamp::fitted(t).slots();
        let dark = t.dark;
        let mut steps = Vec::new();
        for i in 1..7 {
            let (base, bright) = (
                crate::oklch::from_srgb(slots[i]).l,
                crate::oklch::from_srgb(slots[i + 8]).l,
            );
            let step = if dark { bright - base } else { base - bright };
            assert!(
                step > 0.0,
                "{}: {} is not brighter than {} ({step:+.3} lightness)",
                id.as_str(),
                NAMES[i + 8],
                NAMES[i]
            );
            steps.push(step);
        }
        let (lo, hi) = steps
            .iter()
            .fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
        assert!(
            hi - lo < 0.02,
            "{}: the bright step ranges {lo:.3}..{hi:.3} across hues — \
             'bright' means something different per colour",
            id.as_str()
        );
    }
}

/// The coloured phosphor tubes must stay monochrome. Spreading their slots by
/// hue would separate them beautifully and destroy the theme — the same
/// mistake Phase 1 made when it inferred CRT ink from the page and got grey.
///
/// Which tubes those are comes from the ramp, not from `is_crt()`:
/// `crt-paperwhite` is a *white* phosphor and shows real hues.
#[test]
fn a_coloured_phosphor_tube_keeps_one_hue() {
    let mut checked = 0;
    for id in ALL_THEMES {
        let ramp = AnsiRamp::fitted(id.theme());
        let AnsiMode::Monochrome { hue, .. } = ramp.mode() else {
            continue;
        };
        checked += 1;
        let slots = ramp.slots();
        for i in 1..7 {
            let h = crate::oklch::from_srgb(slots[i]).h;
            let off = ((h - hue + 540.0) % 360.0 - 180.0).abs();
            assert!(
                off < 12.0,
                "{}: ansi[{i}] sits {off:.0}° off the phosphor — the tube is no \
                 longer monochrome",
                id.as_str()
            );
        }
    }
    assert_eq!(
        checked, 4,
        "expected four coloured phosphor tubes, found {checked} — if a tube \
         changed mode, say so here"
    );
}

/// The same guard the text ladder has: what ships *is* what the ramp derives,
/// so the presets and the system cannot quietly diverge again.
#[test]
fn every_shipped_ansi_palette_is_what_the_ramp_produces() {
    let mut off: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let slots = AnsiRamp::fitted(t).slots();
        for i in 0..16 {
            let (got, have) = (slots[i], t.ansi[i]);
            let d = |a: u8, b: u8| (a as i16 - b as i16).abs();
            if d(got.0, have.0) > 1 || d(got.1, have.1) > 1 || d(got.2, have.2) > 1 {
                off.push(format!(
                    "{} ansi[{i}] ({}): shipped {have:?}, ramp says {got:?}",
                    id.as_str(),
                    NAMES[i]
                ));
            }
        }
    }
    assert!(
        off.is_empty(),
        "{} of {} shipped slots are not what the ramp derives:\n  {}",
        off.len(),
        ALL_THEMES.len() * 16,
        off.join("\n  ")
    );
}
