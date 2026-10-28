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
            "dark \u{2192} light \u{2192} crt \u{2192} glass \u{2192} neon \u{2192} classic \u{2192} purple"
        ),
        "{msg}"
    );
    assert!(msg.contains("every 10 minutes"), "{msg}");
    assert!(msg.contains("now glass (nebula), next neon"), "{msg}");
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::NeonDark);
    assert!(msg.contains("now neon (neon-dark), next classic"), "{msg}");
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Gruvbox);
    assert!(msg.contains("now classic (gruvbox), next purple"), "{msg}");
    // The walk wraps: after purple comes dark again.
    let msg = report("auto", Some(RandomMode::Auto), ThemeId::Amethyst);
    assert!(msg.contains("now purple (amethyst), next dark"), "{msg}");
}
