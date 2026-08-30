use super::*;
use crate::md::MdStyle;

fn plain(text: &str) -> MdSpan {
    MdSpan {
        text: text.into(),
        style: MdStyle::default(),
        link: None,
    }
}

#[test]
fn extract_round_trips_the_sentinel() {
    for checked in [false, true] {
        let spans = vec![sentinel(checked), plain("text")];
        let (task, rest) = extract(spans);
        assert_eq!(task, Some(checked));
        assert_eq!(rest, vec![plain("text")]);
    }
}

#[test]
fn extract_leaves_plain_items_alone() {
    let (task, rest) = extract(vec![plain("[x]"), plain(" not a marker")]);
    assert_eq!(task, None, "authored bracket text is not a sentinel");
    assert_eq!(rest.len(), 2);
}

#[test]
fn extract_strips_stray_sentinels_without_claiming_them() {
    // Only reachable past the nesting cap, where lists fold flat: the state
    // is dropped, the sentinel must not leak as literal text.
    let (task, rest) = extract(vec![plain("a"), sentinel(true)]);
    assert_eq!(task, None);
    assert_eq!(rest, vec![plain("a")]);
}

#[test]
fn bullet_covers_all_four_shapes() {
    assert_eq!(bullet(Some(true), None, 0), "\u{2713} ");
    assert_eq!(bullet(Some(false), None, 0), "\u{2610} ");
    assert_eq!(bullet(None, Some(3), 0), "3. ");
    assert_eq!(bullet(None, None, 0), "\u{2022} ");
}

/// Two spaces of indent is not enough to tell a sub-point from its point;
/// the glyph has to change too. It cycles rather than running out.
#[test]
fn each_nesting_level_gets_its_own_bullet() {
    let at = |d| bullet(None, None, d);
    assert_ne!(at(0), at(1));
    assert_ne!(at(1), at(2));
    assert_ne!(at(0), at(2));
    assert_eq!(at(0), at(3), "the pool cycles rather than running out");
    // A checkbox and an ordinal say their own level; only the plain bullet
    // has nothing else to say it with.
    assert_eq!(bullet(Some(false), None, 1), bullet(Some(false), None, 0));
    assert_eq!(bullet(None, Some(2), 1), bullet(None, Some(2), 0));
}
