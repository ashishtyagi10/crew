use super::{for_value, hex_chip};
use crew_theme::ThemeId;

/// On a tube that burns, no coloured chip is bright enough for the filament
/// to turn white: its brightest channel stays at half, and its hue with it.
/// A white or grey chip, with no hue to lose, is left as it is.
#[test]
fn on_a_burning_tube_a_chip_keeps_its_hue() {
    let _g = crate::app::theme_test_guard();
    for tube in [ThemeId::CrtAmber, ThemeId::CrtGreen] {
        crew_theme::set_theme(tube);
        let mut chips = for_value("/theme", "glass-clear");
        chips.extend(for_value("/theme", "crt"));
        chips.extend(hex_chip("#ff7da5"));
        assert!(chips.len() > 4);
        for c in &chips {
            let (top, low) = (
                c.fg.0.max(c.fg.1).max(c.fg.2),
                c.fg.0.min(c.fg.1).min(c.fg.2),
            );
            assert!(top <= 127 || top - low < 24, "{tube:?}: {:?}", c.fg);
        }
        let pink = hex_chip("#ff7da5").unwrap().fg;
        assert!(pink.0 > pink.1 && pink.0 > pink.2, "still pink: {pink:?}");
        assert_eq!(
            hex_chip("#f0f0f0").unwrap().fg,
            (240, 240, 240),
            "a white stays white"
        );
    }
    crew_theme::set_theme(ThemeId::PaperDark);
    assert_eq!(
        hex_chip("#ff7da5").unwrap().fg,
        (255, 125, 165),
        "paper is as it was"
    );
}
