use super::*;

#[test]
fn a_plain_mode_reports_only_itself() {
    assert_eq!(
        report("light", Some(RandomMode::Light), ThemeId::PaperLight),
        "theme: light"
    );
    assert_eq!(report("blossom", None, ThemeId::Blossom), "theme: blossom");
}

#[test]
fn auto_names_the_look_on_screen_and_the_next_one() {
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Nebula);
    assert!(
        msg.contains(
            "dark \u{2192} light \u{2192} crt (all 2) \u{2192} glass (all 4) \u{2192} neon \u{2192} classic (all 5) \u{2192} purple \u{2192} prism"
        ),
        "{msg}"
    );
    assert!(msg.contains("every 10 minutes"), "{msg}");
    // Every look is shown palette by palette: glass's four in turn, then neon.
    assert!(
        msg.contains("now glass (nebula), next glass (blossom)"),
        "{msg}"
    );
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::GlassNight);
    assert!(msg.contains("now glass (glass-night), next neon"), "{msg}");
    // The classics come one by one, from the first, and each names the next.
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::NeonDark);
    assert!(
        msg.contains("now neon (neon-dark), next classic (tokyo-night)"),
        "{msg}"
    );
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Gruvbox);
    assert!(
        msg.contains("now classic (gruvbox), next classic (monokai)"),
        "{msg}"
    );
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Monokai);
    assert!(msg.contains("now classic (monokai), next purple"), "{msg}");
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Amethyst);
    assert!(msg.contains("now purple (amethyst), next prism"), "{msg}");
    // Then the neumorphic four, one by one, and the walk wraps to dark.
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Prism);
    assert!(
        msg.contains("now prism (prism), next soft (porcelain)"),
        "{msg}"
    );
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Slate);
    assert!(msg.contains("now soft (slate), next dark"), "{msg}");
}
