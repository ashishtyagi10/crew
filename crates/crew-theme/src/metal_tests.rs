use crate::*;

/// `metal` is a look of two palettes, gunmetal `mercury` and aluminium
/// `silver`: its names rotate between them, each pins itself by name, and
/// the picker lists both.
#[test]
fn metal_is_a_look_of_mercury_and_silver() {
    let _g = crate::test_guard();
    for name in [
        "metal",
        "liquid-metal",
        "liquidmetal",
        "liquid metal",
        "chrome",
        "METAL",
    ] {
        assert_eq!(
            parse_selection(name),
            Some(Selection::Mode(RandomMode::Metal)),
            "{name}"
        );
    }
    assert_eq!(RandomMode::Metal.as_str(), "metal");
    assert_eq!(RandomMode::Metal.sole_palette(), None);
    for id in [ThemeId::Mercury, ThemeId::Silver] {
        assert_eq!(id.mode(), RandomMode::Metal);
        assert_eq!(ThemeId::from_name(id.as_str()), Some(id));
        assert_eq!(parse_selection(id.as_str()), Some(Selection::Fixed(id)));
        assert!(pinnable_themes().any(|t| t == id), "{}", id.as_str());
    }
    assert!(ThemeId::Mercury.theme().dark && !ThemeId::Silver.theme().dark);
    apply_selection(Selection::Mode(RandomMode::Metal), 1_000);
    assert_eq!(current_id().mode(), RandomMode::Metal);
    apply_selection(Selection::Fixed(ThemeId::PaperDark), 0);
}

/// The metal is the panes, not a scene behind them (the user, 2026-11-17:
/// "actual metal look, instead of black and just some baloon in it"): both
/// palettes are relief cards, solid and unframed, brushed at full strength,
/// and neither brings a 3D scene.
#[test]
fn the_panes_are_brushed_metal_plates() {
    for id in [ThemeId::Mercury, ThemeId::Silver] {
        let t = id.theme();
        assert_eq!(t.relief(), 1.0, "{}", id.as_str());
        assert_eq!(t.metal(), 1.0, "{}", id.as_str());
        assert_eq!(t.liquid.map(|l| l.body), Some(1.0), "{}", id.as_str());
        assert_eq!(id.scene(), None, "{}", id.as_str());
    }
    for id in ALL_THEMES {
        if id.mode() != RandomMode::Metal {
            assert_eq!(id.theme().metal(), 0.0, "{}", id.as_str());
        }
    }
}

/// Mercury's page is gunmetal (nearly grey, no warm cast) and its accent a
/// warm metal, rose gold. Silver's page is a cool aluminium, blue a hair
/// above red.
#[test]
fn the_pages_are_metal_greys() {
    let (r, g, b) = ThemeId::Mercury.theme().page_bg;
    assert!(
        b >= r && b - r <= 6 && g.abs_diff(r) <= 4,
        "mercury page {:?} is not gunmetal",
        (r, g, b)
    );
    let (r, g, b) = ThemeId::Mercury.theme().accent_default;
    assert!(
        r > g && r > b,
        "mercury's accent {:?} is not rose gold",
        (r, g, b)
    );
    let (r, g, b) = ThemeId::Silver.theme().page_bg;
    assert!(
        b > r && b - r <= 14 && r >= 200,
        "silver page {:?} is not aluminium",
        (r, g, b)
    );
}

/// The words on a brushed plate: every ink role read against the face the
/// shader draws ([`metal_face`]). Where the plate rests, every role holds the
/// floor it holds on its page (7:1 for the ink, 4.5:1 for muted text, 3:1 for
/// the quiet roles and the signals). At the two ends of the swing (the
/// sheen's peak, the grain's darkest hairline) the reading text still holds:
/// the ink at 7:1, muted text at 4.5:1, and the quiet roles at 2.5:1, a dip
/// only as wide as the sheen streak.
#[test]
fn every_ink_reads_on_the_brushed_face() {
    for id in [ThemeId::Mercury, ThemeId::Silver] {
        let t = id.theme();
        let quiet = [
            ("legend_off", t.legend_off),
            ("dim", t.dim),
            ("placeholder", t.placeholder),
            ("hint_fg", t.hint_fg),
            ("accent", t.accent_default),
            ("activity", t.activity),
            ("bell", t.bell),
            ("status", t.status_fg),
        ];
        for (k, end) in [(0.0, false), (1.0, true), (-1.0, true)] {
            let face = metal_face(t.page_bg, t.dark, k);
            let at = |c| contrast_ratio(c, face);
            let n = id.as_str();
            assert!(at(t.ink) >= 7.0, "{n}: ink {:.2} on {face:?}", at(t.ink));
            assert!(
                at(t.text_muted) >= 4.5,
                "{n}: muted {:.2} on {face:?}",
                at(t.text_muted)
            );
            let floor = if end { 2.5 } else { 3.0 };
            for (role, c) in quiet {
                assert!(at(c) >= floor, "{n}: {role} {:.2} on {face:?}", at(c));
            }
        }
    }
}

/// A dark plate rests lighter than its page (steel at the page's depth read
/// as black), and a light one at its page.
#[test]
fn a_dark_plate_rests_a_step_above_its_page() {
    let t = ThemeId::Mercury.theme();
    let face = metal_face(t.page_bg, true, 0.0);
    assert!(face.0 >= 40 && face.0 > t.page_bg.0, "{face:?}");
    let s = ThemeId::Silver.theme();
    assert_eq!(metal_face(s.page_bg, false, 0.0), s.page_bg);
}
