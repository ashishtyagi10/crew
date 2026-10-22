use super::{backdrop, holo_glass};
use crate::ThemeId;

#[test]
fn only_the_neon_dark_page_has_a_grid() {
    for id in crate::ALL_THEMES {
        assert_eq!(
            backdrop(id.theme()).is_some(),
            id == ThemeId::NeonDark,
            "{id:?}"
        );
    }
}

/// Every signature colour on the holographic page — a pane's legend, an
/// agent's name, a tag — is a cyan, a blue or a violet: no red, yellow,
/// green or pink left over from the terminal's palette.
#[test]
fn the_signatures_stay_in_the_neon_family() {
    let t = ThemeId::NeonDark.theme();
    for slot in 0..12 {
        let h = crate::oklch::from_srgb(crate::slot_color(slot, t)).h;
        assert!((190.0..=315.0).contains(&h), "slot {slot}: hue {h:.0}");
    }
}

/// The slab glows rather than shades, its frame's light reaches into it,
/// and its halo is the accent's blue, not grey.
#[test]
fn the_holo_slab_glows_in_its_own_blue() {
    let t = ThemeId::NeonDark.theme();
    let g = holo_glass(t, 1.0);
    assert_eq!(g, crate::glass_style_for(t));
    assert!(g.glow == 1.0 && g.edge_glow > 0.1 && g.gloss > 0.0);
    let (r, gr, b) = g.tint;
    assert!(b > r && b > gr, "blue smoke: {:?}", g.tint);
    assert!(holo_glass(t, 0.0).edge_glow == 0.0, "holo 0 lights nothing");
}
