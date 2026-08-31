use crate::app::CrewApp;

#[test]
fn first_frame_records_without_dipping() {
    let mut app = CrewApp::default();
    app.theme_fade_tick(1_000);
    assert_eq!(app.theme_fade(1_001), None, "launch must not fade");
}

#[test]
fn a_theme_change_starts_the_fade_then_it_decays_to_none() {
    let mut app = CrewApp::default();
    app.theme_fade_tick(1_000);
    // Simulate a switch by lying about what was seen — equivalent to the
    // active id changing between frames, without touching the global.
    let other = crew_theme::ALL_THEMES
        .into_iter()
        .find(|&t| Some(t) != app.theme_seen)
        .unwrap();
    app.theme_seen = Some(other);
    app.theme_fade_tick(2_000);
    let a0 = app.theme_fade(2_001).expect("fade up right after");
    assert!(a0 > 0.9, "starts near-full old frame, got {a0}");
    let a_mid = app.theme_fade(2_150).expect("still fading");
    assert!(a_mid < a0, "fade must decay: {a_mid} !< {a0}");
    assert_eq!(app.theme_fade(2_600), None, "settled past FADE_MS");
    // And the fade registers with the redraw scheduler while live.
    assert!(app.theme_fade_anim.live(2_100));
    assert!(!app.theme_fade_anim.live(2_600));
}

#[test]
fn an_unchanged_theme_never_restamps() {
    let mut app = CrewApp::default();
    app.theme_fade_tick(1_000);
    app.theme_fade_tick(2_000);
    app.theme_fade_tick(3_000);
    assert_eq!(app.theme_fade(3_001), None);
}
