//! The question these have to answer is not "does the ramp produce colours" —
//! it is "does it produce *these* colours". This work's one rule is that crew
//! must not end up looking different, so the drift against every shipped
//! palette is measured, bounded, and printed when it fails.
use super::*;
use crate::oklch::distance;
use crate::{contrast_ratio, ALL_THEMES};

/// One derived role: its name, how the ramp produces it, and where the shipped
/// preset keeps it.
type RoleCase = (
    &'static str,
    fn(&Ramp) -> (u8, u8, u8),
    fn(&crate::Theme) -> (u8, u8, u8),
);

/// A derived role with no shipped counterpart to compare against: its name,
/// how the ramp produces it, and where its target sits in the ladder.
type Derived = (&'static str, fn(&Ramp) -> (u8, u8, u8), fn(&House) -> f32);

#[test]
fn every_shipped_ladder_is_what_the_ramp_produces() {
    let roles: [RoleCase; 7] = [
        ("ink", |r| r.ink(), |t| t.ink),
        ("text_muted", |r| r.text_muted(), |t| t.text_muted),
        ("legend_off", |r| r.legend_off(), |t| t.legend_off),
        ("dim", |r| r.dim(), |t| t.dim),
        ("hint_fg", |r| r.hint_fg(), |t| t.hint_fg),
        ("placeholder", |r| r.placeholder(), |t| t.placeholder),
        ("border_normal", |r| r.border_normal(), |t| t.border_normal),
    ];
    let mut off: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let ramp = Ramp::fitted(t);
        for (name, derive, current) in roles {
            let (got, have) = (derive(&ramp), current(t));
            // One 8-bit code of slack per channel. The ramp reads its own hue
            // and chroma back off the shipped `ink`, so re-deriving squeezes
            // through sRGB quantisation once more; anything larger means the
            // presets have been edited away from the ramp by hand.
            let d = |a: u8, b: u8| (a as i16 - b as i16).abs();
            if d(got.0, have.0) > 1 || d(got.1, have.1) > 1 || d(got.2, have.2) > 1 {
                off.push(format!(
                    "{} {name}: shipped {have:?}, ramp says {got:?}",
                    id.as_str()
                ));
            }
        }
    }
    assert!(
        off.is_empty(),
        "{} of {} shipped roles are not what the ramp derives — the palettes \
         and the system have diverged, which is the state this work exists to \
         end:\n  {}",
        off.len(),
        ALL_THEMES.len() * roles.len(),
        off.join("\n  ")
    );
}

/// The point of the exercise: after derivation the same role means the same
/// thing everywhere in a ladder. Today these span up to 2.05x (module docs).
///
/// One documented exception, and it is the trade the lightness cap buys: a
/// theme whose page is lighter than its ladder's others cannot reach the house
/// ratio without going near-white, so it stops at [`House::max_l`] and sits a
/// little under. Those themes are held to a different assertion — that they
/// are at the cap — rather than waved through.
#[test]
fn a_role_means_the_same_thing_in_every_theme_of_a_ladder() {
    let roles: [Derived; 7] = [
        ("ink", |r| r.ink(), |h| h.ink),
        ("text_muted", |r| r.text_muted(), |h| h.text_muted),
        ("legend_off", |r| r.legend_off(), |h| h.legend_off),
        ("dim", |r| r.dim(), |h| h.dim),
        ("hint_fg", |r| r.hint_fg(), |h| h.hint_fg),
        ("placeholder", |r| r.placeholder(), |h| h.placeholder),
        ("border_normal", |r| r.border_normal(), |h| h.border_normal),
    ];
    let mut ladders: Vec<&str> = ALL_THEMES
        .iter()
        .map(|id| Ramp::fitted(id.theme()).house().name)
        .collect();
    ladders.sort_unstable();
    ladders.dedup();
    // Two in use since `crt-paperwhite` retired. `HOUSE_CRT_WHITE` is kept
    // rather than deleted: the rule that a white phosphor pays none of a
    // coloured one's contrast cost is correct and cheap, and a future white
    // tube would otherwise silently derive as monochrome.
    assert_eq!(
        ladders,
        vec!["crt (coloured phosphor)", "paper/modern"],
        "the ladders in use changed"
    );

    for (name, derive, target) in roles {
        for ladder in &ladders {
            let mut free: Vec<f32> = Vec::new();
            for id in ALL_THEMES
                .iter()
                .filter(|id| Ramp::fitted(id.theme()).house().name == *ladder)
            {
                let t = id.theme();
                let r = Ramp::fitted(t);
                let c = derive(&r);
                if r.ceiling_bound(target(&r.house())) {
                    // The cap bound here. Assert that, and exclude it from the
                    // consistency band it cannot meet by construction.
                    assert!(
                        contrast_ratio(c, t.page_bg) >= 10.0,
                        "{}: capped {name} fell below the ink floor",
                        id.as_str()
                    );
                } else {
                    free.push(contrast_ratio(c, t.page_bg));
                }
            }
            if free.len() < 2 {
                continue;
            }
            let (lo, hi) = free
                .iter()
                .fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
            assert!(
                hi / lo < 1.06,
                "{name} spans {lo:.2}..{hi:.2} ({:.2}x) within the {ladder} \
                 ladder ({} uncapped themes) — the ramp has not made it \
                 consistent",
                hi / lo,
                free.len()
            );
        }
    }
}

/// The ladder must stay ordered, or the visual hierarchy inverts somewhere.
#[test]
fn the_ladder_is_ordered_on_every_page() {
    for id in ALL_THEMES {
        let t = id.theme();
        let r = Ramp::fitted(t);
        let cr = |c| contrast_ratio(c, t.page_bg);
        let steps = [
            ("ink", cr(r.ink())),
            ("text_muted", cr(r.text_muted())),
            ("legend_off", cr(r.legend_off())),
            ("hint_fg", cr(r.hint_fg())),
            ("placeholder", cr(r.placeholder())),
            ("border_normal", cr(r.border_normal())),
        ];
        for w in steps.windows(2) {
            assert!(
                w[0].1 > w[1].1,
                "{}: {} ({:.2}) must sit above {} ({:.2})",
                id.as_str(),
                w[0].0,
                w[0].1,
                w[1].0,
                w[1].1
            );
        }
    }
}

/// Every derived role must clear the floor the independent suite asserts, on
/// every page — otherwise the ramp trades consistency for legibility.
#[test]
fn every_derived_role_clears_the_contrast_floor() {
    for id in ALL_THEMES {
        let t = id.theme();
        let r = Ramp::fitted(t);
        let bg = t.page_bg;
        let checks = [
            ("ink", r.ink(), 10.0),
            ("text_muted", r.text_muted(), 7.0),
            ("legend_off", r.legend_off(), 3.0),
            ("hint_fg", r.hint_fg(), 2.5),
            ("placeholder", r.placeholder(), 2.3),
            ("border_normal", r.border_normal(), 1.45),
        ];
        for (name, c, floor) in checks {
            let got = contrast_ratio(c, bg);
            assert!(
                got >= floor,
                "{}: derived {name} = {got:.3}, below the {floor} floor",
                id.as_str()
            );
        }
    }
}

/// Neutrals inherit the page's temperature, or every theme's greys collapse
/// into the same ladder and SEPIA stops being warm.
#[test]
fn neutrals_keep_the_page_s_temperature() {
    let warm = Ramp::for_page(crate::SEPIA_DARK.page_bg).text_muted();
    let cool = Ramp::for_page(crate::NEBULA.page_bg).text_muted();
    let d = distance(warm, cool);
    assert!(
        d > 0.01,
        "SEPIA and AURORA derive the same muted grey (Δ {d:.4}) — the ramp has \
         flattened the warm/cool cast that distinguishes them"
    );
    // …but not so far that it reads as coloured text rather than as ink.
    for page in [crate::SEPIA_DARK.page_bg, crate::CRT_GREEN.page_bg] {
        let c = crate::oklch::from_srgb(Ramp::for_page(page).ink()).c;
        assert!(
            c <= NEUTRAL_CHROMA_CAP + 1e-3,
            "ink derived from {page:?} has chroma {c:.4} — that is coloured \
             text, not ink"
        );
    }
}

/// The lightness cap, tested directly rather than through whichever theme
/// happens to trip it.
///
/// `graphite` was the only palette that hit it, and `graphite` was retired in
/// the 24→9 cut — at which point the roster-driven assertion that "something
/// is capped" started failing and asked the right question: is the cap doing
/// nothing? No. It is a **ceiling, not a feature**: one that is never reached
/// is working. But a guard nothing exercises is a guard nothing tests, so this
/// drives it with the page that motivated it.
#[test]
fn the_lightness_ceiling_holds_a_page_that_would_go_near_white() {
    // `graphite`'s page. It sat at L 0.231 against its pool's 0.10..0.19, so
    // reaching the house contrast wanted an ink of L 0.986 — `(250, 250, 252)`,
    // effectively white.
    const GRAPHITE_PAGE: (u8, u8, u8) = (32, 28, 27);
    let ramp = Ramp::new(GRAPHITE_PAGE, Ink::of((226, 226, 228)), HOUSE);

    assert!(
        ramp.ceiling_bound(HOUSE.ink),
        "this page no longer pushes past the ceiling — the fixture has drifted \
         and the cap is untested again"
    );
    let ink = ramp.ink();
    let l = crate::oklch::from_srgb(ink).l;
    assert!(
        l <= HOUSE.max_l + 1e-3,
        "derived ink {ink:?} sits at L {l:.4}, above the {} ceiling — that is \
         the glare the cap exists to prevent",
        HOUSE.max_l
    );
    // …and it is still comfortably legible after being held back.
    assert!(
        contrast_ratio(ink, GRAPHITE_PAGE) >= 10.0,
        "capping dropped the ink below the contrast floor"
    );
}
