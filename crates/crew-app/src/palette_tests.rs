use super::*;

#[test]
fn pack_unpack_round_trips() {
    for rgb in [(0, 255, 160), (255, 255, 255), (0, 0, 0), (18, 200, 7)] {
        assert_eq!(unpack(pack(rgb)), rgb);
    }
}

#[test]
fn parse_hex_accepts_with_and_without_hash() {
    assert_eq!(parse_hex("#00ffa0"), Some((0, 255, 160)));
    assert_eq!(parse_hex("00FFA0"), Some((0, 255, 160)));
    assert_eq!(parse_hex("  #123456 "), Some((0x12, 0x34, 0x56)));
}

#[test]
fn parse_hex_rejects_bad_input() {
    assert_eq!(parse_hex(""), None);
    assert_eq!(parse_hex("#fff"), None); // shorthand not supported
    assert_eq!(parse_hex("#gggggg"), None);
    assert_eq!(parse_hex("0xffaa00"), None);
}

#[test]
fn set_then_raw_accent_round_trips() {
    // Serialise with any other test that reads the accent global.
    let _g = crate::palette::test_guard();
    set_accent((10, 20, 30));
    assert_eq!(raw_accent(), (10, 20, 30));
    set_accent(DEFAULT_ACCENT); // restore so other tests see the default
    assert_eq!(raw_accent(), DEFAULT_ACCENT);
}

/// The defect: crew's own brand green, and the value anyone carries over
/// from a dark theme, reads at 1.2 against every light page in the set.
/// Every theme's own default already clears the floor and must come back
/// untouched — the floor is for the colour the *user* can set.
#[test]
fn a_user_accent_is_floored_against_the_page_it_lands_on() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::contrast::text_floor();
    let mut bad: Vec<String> = Vec::new();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let page = crew_theme::theme().page_bg;

        set_accent(DEFAULT_ACCENT);
        let r = crew_theme::contrast_ratio(accent(), page);
        if r < floor {
            bad.push(format!("{}: crew green floored to {r:.2}", id.as_str()));
        }

        let d = crew_theme::theme().accent_default;
        set_accent(d);
        if accent() != d {
            bad.push(format!("{}: its own default was moved", id.as_str()));
        }
    }
    set_accent(DEFAULT_ACCENT);
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}
/// Focus in a form is drawn by swapping muted ink for accent ink, so the
/// two have to be tellable apart. `accent()` is floored against the PAGE
/// — it says the colour can be read, not that it can be told from the one
/// it stands in for. Measured before this floor: sepia-dark **1.04** and
/// crt-violet **1.06**, i.e. the same lightness, so the focused input's
/// border differed by hue alone — and on a single-phosphor tube, not at
/// all.
#[test]
fn the_focus_accent_can_be_told_from_the_ink_it_replaces() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let mut tubes = 0;
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let t = crew_theme::theme();
        let want = match t.is_tube() {
            true => {
                tubes += 1;
                super::TUBE_FOCUS_FLOOR
            }
            false => super::FOCUS_FLOOR,
        };
        let got = crew_theme::contrast_ratio(super::focus_accent(), t.text_muted);
        assert!(
            got >= want,
            "{}: focus accent vs muted = {got:.2} (want {want})",
            id.as_str(),
        );
        // …and it must still read on the page it is drawn on.
        let page = crew_theme::contrast_ratio(super::focus_accent(), t.page_bg);
        assert!(
            page >= crew_theme::contrast::mark_floor(),
            "{}: and it stays legible: {page:.2}",
            id.as_str(),
        );
    }
    assert_eq!(tubes, 4, "every tube was actually checked");
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
