use super::*;
use crate::{contrast_ratio, ThemeId, ALL_THEMES};

/// The same contract the ramp and the highlight wash hold: what ships IS what
/// the derivation produces.
#[test]
fn every_shipped_alarm_is_what_the_derivation_produces() {
    let mut off: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let got = alarm_for(id);
        if got != id.theme().bell {
            off.push(format!(
                "{}: shipped {:?}, the derivation says {got:?}",
                id.as_str(),
                id.theme().bell
            ));
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n  "));
}

/// The defect: on `paper-dark` and `sepia-dark` the attention glyph and the
/// status line were literally one colour (Δ 0.000), and on `paper-light` and
/// `sepia-light` within one visible step of it.
#[test]
fn the_alarm_is_a_different_colour_from_the_status_on_every_coloured_page() {
    for id in ALL_THEMES.into_iter().filter(|id| !id.is_crt()) {
        let t = id.theme();
        let d = oklch::distance(t.status_fg, t.bell);
        assert!(
            d >= FLOOR - 1e-3,
            "{}: bell is only Δ {d:.4} from status_fg (floor {FLOOR}) — \
             a pane that needs you looks like a pane that is merely working",
            id.as_str()
        );
    }
}

/// The exemption is for phosphor tubes and NOTHING else. Without this a new
/// coloured palette could ship an amber-on-amber bell simply by being added,
/// and the test above would pass because it never looked at it.
#[test]
fn only_the_tubes_are_exempt_from_the_separation() {
    let exempt: Vec<&str> = ALL_THEMES
        .into_iter()
        .filter(|id| oklch::distance(id.theme().status_fg, id.theme().bell) < FLOOR)
        .map(ThemeId::as_str)
        .collect();
    assert_eq!(
        exempt,
        // `crt-violet` is a tube that CAN separate: its phosphor leaves room
        // for a warm pink alarm the other three have nowhere to put.
        vec!["crt-green", "crt-amber", "crt-blue"],
        "the set of palettes that cannot separate their alarm has changed"
    );
}

/// Why the tubes are exempt, asserted rather than asserted-in-prose: their
/// alert slot sits on the SAME hue as their status, because a phosphor tube
/// has exactly one. Rotating it away is the mistake the ramp's docs record.
#[test]
fn a_tube_has_no_second_hue_to_move_the_alarm_to() {
    for id in ALL_THEMES.into_iter().filter(|id| id.is_crt()) {
        let t = id.theme();
        let h = |c: (u8, u8, u8)| oklch::from_srgb(c).h;
        let apart = (h(t.ansi[9]) - h(t.status_fg))
            .abs()
            .min(360.0 - (h(t.ansi[9]) - h(t.status_fg)).abs());
        assert!(
            apart < 25.0,
            "{}: alert slot is {apart:.0}° off the status hue — this tube DOES \
             have somewhere to put an alarm, so it should not be exempt",
            id.as_str()
        );
    }
}

/// An alarm that is quieter than the status it interrupts is not an alarm.
///
/// Asserted against the DERIVATION rather than the shipped presets: once the
/// derived values are baked in, "was this one derived?" is unanswerable by
/// inspection (that is exactly what the parity test guarantees). So each
/// coloured palette is handed a colliding alarm — the state it shipped in
/// before this work — and what comes back has to be as loud as the status.
#[test]
fn a_derived_alarm_is_never_quieter_than_the_status_it_interrupts() {
    for id in ALL_THEMES.into_iter().filter(|id| !id.is_crt()) {
        let t = id.theme();
        // The worst collision there is: the alarm IS the status.
        let got = alarm(t.page_bg, t.status_fg, t.ansi[9], t.status_fg);
        let (bell, status) = (
            contrast_ratio(got, t.page_bg),
            contrast_ratio(t.status_fg, t.page_bg),
        );
        assert!(
            bell >= status * 0.97,
            "{}: a derived bell is {bell:.2}:1 against the page where status \
             is {status:.2}:1",
            id.as_str()
        );
        assert!(
            oklch::distance(t.status_fg, got) >= FLOOR - 1e-3,
            "{}: deriving from a total collision only reached Δ {:.4}",
            id.as_str(),
            oklch::distance(t.status_fg, got)
        );
    }
}

/// Whatever a palette declares, the alarm has to be legible on the page.
/// `blossom` is the floor case at 4.84:1 — it separates its alarm by hue, at
/// its own choice of loudness, and this is the line under that choice.
#[test]
fn every_alarm_clears_the_page_regardless_of_who_chose_it() {
    for id in ALL_THEMES {
        let t = id.theme();
        let cr = contrast_ratio(t.bell, t.page_bg);
        assert!(
            cr >= 4.5,
            "{}: bell is {cr:.2}:1 against the page (need >= 4.5)",
            id.as_str()
        );
    }
}

/// A palette that already separates the two is left exactly as it is —
/// `nebula` and `blossom` did this by hand and by taste, and the derivation
/// exists to catch up with them, not to overwrite them.
#[test]
fn a_palette_that_already_separates_is_untouched() {
    for id in [ThemeId::Nebula, ThemeId::Blossom] {
        assert_eq!(alarm_for(id), id.theme().bell);
    }
    // And the core maths honours that independently of the theme roster.
    let far = (255, 0, 0);
    assert_eq!(alarm((0, 0, 0), (0, 255, 0), (200, 30, 30), far), far);
}

/// One signal role: its name, how to read it off a palette, and how far its
/// page contrast may spread inside one appearance.
type Band = (&'static str, fn(&crate::Theme) -> (u8, u8, u8), f32);

/// Every signal role and its band. See the module docs for why the bound is
/// per-appearance and why `accent_default` gets its own.
const BANDS: [Band; 5] = [
    ("status_fg", |t| t.status_fg, 1.8),
    ("bell", |t| t.bell, 1.8),
    ("broadcast", |t| t.broadcast, 1.8),
    ("activity", |t| t.activity, 1.8),
    // Monochrome is `paper-dark`'s identity: its near-white accent measures
    // 17.51 where `nebula`'s orchid is 8.22. Named, not averaged away.
    ("accent_default", |t| t.accent_default, 2.45),
];

/// The three appearances a palette can have, as the pools the bands apply
/// within. A tube is its own case: everything on it is bright by nature.
fn pools() -> [(&'static str, Vec<ThemeId>); 3] {
    let pick = |f: fn(ThemeId) -> bool| ALL_THEMES.into_iter().filter(|id| f(*id)).collect();
    [
        ("dark", pick(|id| !id.is_crt() && id.theme().dark)),
        ("light", pick(|id| !id.is_crt() && !id.theme().dark)),
        ("tube", pick(ThemeId::is_crt)),
    ]
}

/// A signal role means the same thing across the palettes of one appearance.
///
/// This is a tripwire, not a derivation — no colour is chosen from it. It
/// exists because the two defects this module and `highlight` were written to
/// fix had the same shape: a role outside the ramp's contract with nothing
/// measuring it, drifting until someone noticed by eye.
#[test]
fn a_signal_role_holds_its_band_inside_an_appearance() {
    for (pool, ids) in pools() {
        for (role, get, bound) in BANDS {
            let mut v: Vec<(f32, &str)> = ids
                .iter()
                .map(|id| {
                    (
                        contrast_ratio(get(id.theme()), id.theme().page_bg),
                        id.as_str(),
                    )
                })
                .collect();
            v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let spread = v[v.len() - 1].0 / v[0].0;
            assert!(
                spread <= bound,
                "{pool}: {role} spans {spread:.2}x (bound {bound}) — {}",
                v.iter()
                    .map(|(c, n)| format!("{n} {c:.2}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
}

/// A bound nothing comes near is a bound that catches nothing — checked PER
/// ROLE, because a single worst-case check lets any one bound be inflated to
/// nonsense while another role holds the test up. (It did: loosening
/// `status_fg` to 9.9 passed the first version of this.)
///
/// Closest today: `broadcast` on the tubes at 1.58x of a 1.8 bound.
#[test]
fn every_band_is_close_enough_to_the_palettes_to_bite() {
    for (role, get, bound) in BANDS {
        let reach = pools()
            .iter()
            .map(|(_, ids)| {
                let mut v: Vec<f32> = ids
                    .iter()
                    .map(|id| contrast_ratio(get(id.theme()), id.theme().page_bg))
                    .collect();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap());
                v[v.len() - 1] / v[0]
            })
            .fold(0.0f32, f32::max);
        assert!(
            reach / bound > 0.75,
            "{role}: the widest any appearance spreads is {reach:.2}x against \
             a bound of {bound} ({:.0}% of it) — that bound has stopped \
             constraining the palettes",
            reach / bound * 100.0
        );
    }
}
