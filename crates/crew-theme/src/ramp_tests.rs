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

/// A derived role with no shipped counterpart to compare against.
type Derived = (&'static str, fn(&Ramp) -> (u8, u8, u8));

/// Half a rung of crew's own text hierarchy (AURORA `ink` → `text_muted`
/// measures Δ 0.10 — see the scale test in `oklch`). Below this, a role has
/// moved without changing what it is.
const HALF_RUNG: f32 = 0.05;

/// A full rung. Past this a role has moved far enough to be worth naming.
const FULL_RUNG: f32 = 0.10;

/// Roles the ramp deliberately moves more than a full rung, because the
/// palette they came from was out of family.
///
/// This list is the honest form of "crew must not look different". A blanket
/// cap would have been a lie — the whole point is to pull outliers back to the
/// house ladder, and refusing to move anything would mean deriving nothing.
/// Instead every correction is named and reviewed here, and an *unnamed* one
/// fails the test.
///
/// All five are the same story. `dim` across the non-CRT pool spans 2.67..5.37
/// with a median of 4.51; these four modern-light palettes sit at the bottom
/// of that range with an input-bar hint noticeably fainter than every other
/// theme's, and `crt-paperwhite` does the same within its own pool. Light and
/// dark medians were checked separately before accepting this — they agree
/// (dim 4.51 light, 4.63 dark), so this is not a light-page property being
/// flattened, it is four palettes disagreeing with the other twenty.
const EXPECTED_CORRECTIONS: [(&str, &str); 5] = [
    ("daybreak", "dim"),
    ("cirrus", "dim"),
    ("meadow", "dim"),
    ("blossom", "dim"),
    ("crt-paperwhite", "dim"),
];

#[test]
fn the_derived_ladder_stays_close_to_every_shipped_palette() {
    let roles: [RoleCase; 7] = [
        ("ink", |r| r.ink(), |t| t.ink),
        ("text_muted", |r| r.text_muted(), |t| t.text_muted),
        ("legend_off", |r| r.legend_off(), |t| t.legend_off),
        ("dim", |r| r.dim(), |t| t.dim),
        ("hint_fg", |r| r.hint_fg(), |t| t.hint_fg),
        ("placeholder", |r| r.placeholder(), |t| t.placeholder),
        ("border_normal", |r| r.border_normal(), |t| t.border_normal),
    ];
    let mut all: Vec<(f32, &str, &str, String)> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let ramp = Ramp::fitted(t);
        for (name, derive, current) in roles {
            let (got, have) = (derive(&ramp), current(t));
            let d = distance(got, have);
            all.push((
                d,
                id.as_str(),
                name,
                format!("{have:?} → {got:?} (Δ {d:.4})"),
            ));
        }
    }

    // 1. Overall, the look is preserved: the typical role barely moves.
    let mut sorted: Vec<f32> = all.iter().map(|(d, ..)| *d).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];
    assert!(
        median < 0.03,
        "median drift {median:.4} across {} roles — the ramp is redrawing the \
         palettes rather than regularising them",
        sorted.len()
    );

    // 2. Nothing moves beyond recognition, corrections included.
    let worst = all
        .iter()
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .unwrap();
    assert!(
        worst.0 < 0.15,
        "{} {} moved {:.4}, past anything reviewable: {}",
        worst.1,
        worst.2,
        worst.0,
        worst.3
    );

    // 3. Every role that moves more than a rung is one we named.
    let mut moved: Vec<(&str, &str)> = all
        .iter()
        .filter(|(d, ..)| *d > FULL_RUNG)
        .map(|(_, theme, role, _)| (*theme, *role))
        .collect();
    moved.sort_unstable();
    let mut expected = EXPECTED_CORRECTIONS.to_vec();
    expected.sort_unstable();
    assert_eq!(
        moved,
        expected,
        "the set of corrections changed. Unnamed movement past a rung is a \
         regression, not a correction — details:\n  {}",
        all.iter()
            .filter(|(d, ..)| *d > FULL_RUNG)
            .map(|(_, t, r, s)| format!("{t} {r}: {s}"))
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    // 4. And most roles do not move perceptibly at all.
    let steady = all.iter().filter(|(d, ..)| *d <= HALF_RUNG).count();
    assert!(
        steady * 100 / all.len() >= 70,
        "only {steady}/{} roles held within half a rung — too much of the \
         palette is moving",
        all.len()
    );
}

/// The point of the exercise: after derivation the same role means the same
/// thing everywhere. Today these span up to 2.05x (see the module docs).
#[test]
fn a_role_means_the_same_thing_in_every_theme_of_a_pool() {
    let roles: [Derived; 7] = [
        ("ink", |r| r.ink()),
        ("text_muted", |r| r.text_muted()),
        ("legend_off", |r| r.legend_off()),
        ("dim", |r| r.dim()),
        ("hint_fg", |r| r.hint_fg()),
        ("placeholder", |r| r.placeholder()),
        ("border_normal", |r| r.border_normal()),
    ];
    for (name, derive) in roles {
        // Per pool: the CRT ladder genuinely sits lower than the paper one
        // (a phosphor is a coloured ink, and colour costs contrast), so a
        // single band across all 24 would be the wrong invariant.
        for crt in [false, true] {
            let ratios: Vec<f32> = ALL_THEMES
                .iter()
                .filter(|id| id.is_crt() == crt)
                .map(|id| {
                    let t = id.theme();
                    contrast_ratio(derive(&Ramp::fitted(t)), t.page_bg)
                })
                .collect();
            let (lo, hi) = ratios
                .iter()
                .fold((f32::MAX, 0.0f32), |(a, b), &v| (a.min(v), b.max(v)));
            let pool = if crt { "crt" } else { "paper/modern" };
            assert!(
                hi / lo < 1.06,
                "{name} still spans {lo:.2}..{hi:.2} ({:.2}x) across the {pool} \
                 pool — the ramp has not made it consistent",
                hi / lo
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
    let cool = Ramp::for_page(crate::AURORA.page_bg).text_muted();
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
