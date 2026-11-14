//! One shot per palette of a thing every palette draws its own way: what
//! the theme loop of 2026-11-14 looked at, round by round. The glass survey
//! shoots five windows a palette and takes six minutes; these take one.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_welcome_shot -- --ignored`
use crew_theme::ALL_THEMES;

/// The welcome on every palette: its rain falls in the palette's own hue
/// ([`crate::rainink`]).
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_welcome_shot() {
    let _g = crate::app::theme_test_guard();
    for id in ALL_THEMES {
        let name = format!("welcome-{}", id.as_str());
        let sheer = crate::tubesheer::sheer(id.theme());
        if crate::crtglassshot_tests::window_with(&name, id, sheer, &[], &|_| {}).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}

/// The soft palettes' in-card boxes, pressed in as wells
/// ([`crate::reliefwell`]): the settings form beside the to-do list.
///
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew theme_wells_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn theme_wells_shot() {
    let _g = crate::app::theme_test_guard();
    let soft = |id: &crew_theme::ThemeId| id.mode() == crew_theme::RandomMode::Relief;
    for id in ALL_THEMES.into_iter().filter(soft) {
        let name = format!("wells-{}", id.as_str());
        let sheer = crate::tubesheer::sheer(id.theme());
        let cmds = ["/todo", "/settings"];
        if crate::crtglassshot_tests::window_with(&name, id, sheer, &cmds, &|_| {}).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
}
