use super::*;
use crate::{contrast_ratio, ALL_THEMES};

/// Same contract the ramp holds its ladder to: what ships IS what the
/// derivation produces, so a hand-edit to a preset cannot quietly drift the
/// palettes and the system apart again.
#[test]
fn every_shipped_highlight_is_what_the_wash_produces() {
    let mut off: Vec<String> = Vec::new();
    for id in ALL_THEMES {
        let t = id.theme();
        let got = wash(t.page_bg, t.find_hl_bg);
        if got != t.find_hl_bg {
            off.push(format!(
                "{}: shipped {:?} (Δ {:.4}), the wash says {got:?} (Δ {:.4})",
                id.as_str(),
                t.find_hl_bg,
                oklch::distance(t.page_bg, t.find_hl_bg),
                oklch::distance(t.page_bg, got),
            ));
        }
    }
    assert!(
        off.is_empty(),
        "{} of {} highlights are not what the wash derives:\n  {}",
        off.len(),
        ALL_THEMES.len(),
        off.join("\n  ")
    );
}

/// The defect this module exists for: `paper-light` shipped a highlight
/// 1.25:1 against its page and Δ 0.1057 off it — one rung of the text
/// hierarchy, for the thing that is supposed to shout "here".
#[test]
fn a_match_is_equally_findable_on_every_page() {
    for id in ALL_THEMES {
        let t = id.theme();
        let d = oklch::distance(t.page_bg, t.find_hl_bg);
        assert!(
            d >= FLOOR - 1e-3,
            "{}: highlight is only Δ {d:.4} off the page (floor {FLOOR}) — \
             a match you have to hunt for",
            id.as_str()
        );
    }
    // ...and the floor really is the tightest thing the shipped palettes
    // satisfy, so this is a live constraint rather than a number every theme
    // clears by miles. (Raising FLOOR without re-deriving trips the parity
    // test above; this catches the reverse — dropping it to something vacuous.)
    let min = ALL_THEMES
        .iter()
        .map(|id| oklch::distance(id.theme().page_bg, id.theme().find_hl_bg))
        .fold(f32::MAX, f32::min);
    assert!(
        min < FLOOR + 0.01,
        "no palette sits near the floor (tightest is Δ {min:.4} vs floor \
         {FLOOR}) — the floor has stopped constraining anything"
    );
}

/// A wash is only useful if the words on it survive. The tubes are the tight
/// case: `crt-green` ships 7.27:1 and every other theme has more room.
#[test]
fn ink_stays_readable_on_the_highlight() {
    for id in ALL_THEMES {
        let t = id.theme();
        let cr = contrast_ratio(t.ink, t.find_hl_bg);
        assert!(
            cr >= 7.0,
            "{}: ink on the highlight is {cr:.2}:1 (need >= 7.0) — the wash \
             has swallowed the match it is meant to point at",
            id.as_str()
        );
    }
}

/// A palette already past the floor is left alone: the point is to lift the
/// four that were short, not to re-tune the five that agreed.
#[test]
fn the_wash_never_moves_a_palette_that_already_clears_the_floor() {
    let page = (12, 8, 5);
    for declared in [(70, 62, 20), (10, 70, 30), (255, 255, 255)] {
        assert_eq!(wash(page, declared), declared, "{declared:?} was moved");
    }
    // A short one IS moved, along its own hue rather than to some house
    // colour — the mistake the ramp's docs record.
    let short = (30, 24, 14);
    let lifted = wash(page, short);
    assert_ne!(lifted, short);
    assert!((oklch::distance(page, lifted) - FLOOR).abs() < 0.01);
    let hue = |c: (u8, u8, u8)| oklch::from_srgb(c).h;
    assert!(
        (hue(lifted) - hue(short)).abs() < 8.0,
        "lifting turned hue {:.1} into {:.1}",
        hue(short),
        hue(lifted)
    );
}

/// A palette whose highlight IS its page has no direction to scale, and must
/// come back unchanged rather than dividing by zero into a NaN colour.
#[test]
fn a_highlight_equal_to_the_page_is_returned_not_exploded() {
    assert_eq!(wash((12, 8, 5), (12, 8, 5)), (12, 8, 5));
}
