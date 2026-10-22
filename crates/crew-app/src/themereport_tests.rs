use super::*;
use crew_theme::ThemeId;

#[test]
fn a_plain_mode_reports_only_itself() {
    assert_eq!(
        report("light", Some(RandomMode::Light), true, (None, None)),
        "theme: light"
    );
    assert_eq!(
        report("blossom", None, false, (None, None)),
        "theme: blossom"
    );
}

#[test]
fn auto_names_the_live_half_and_the_dormant_one() {
    // The shape this module exists for: a light half pinned to something
    // while macOS is dark. The old report was the bare word "auto", which
    // is why such a pairing read as a no-op.
    let msg = report(
        "auto",
        Some(RandomMode::Auto),
        true,
        (None, Some(Selection::Fixed(ThemeId::Blossom))),
    );
    assert!(msg.contains("OS is dark, serving dark"), "{msg}");
    assert!(msg.contains("the light half is blossom"), "{msg}");
    assert!(msg.contains("turns light"), "{msg}");

    // Flip the appearance and the same pairing is the one on screen.
    let msg = report(
        "auto",
        Some(RandomMode::Auto),
        false,
        (None, Some(Selection::Fixed(ThemeId::Blossom))),
    );
    assert!(msg.contains("OS is light, serving blossom"), "{msg}");
    assert!(msg.contains("the dark half is dark"), "{msg}");
}

#[test]
fn an_unpaired_side_reports_the_pool_it_actually_serves() {
    // `None` is not "nothing" — it is the built-in paper pool, whose name
    // is `dark`/`light`; a pinned palette names itself.
    let msg = report(
        "auto",
        Some(RandomMode::Auto),
        false,
        (Some(Selection::Fixed(ThemeId::CrtGreen)), None),
    );
    assert!(msg.contains("serving light"), "{msg}");
    assert!(msg.contains("the dark half is crt-green"), "{msg}");
}
