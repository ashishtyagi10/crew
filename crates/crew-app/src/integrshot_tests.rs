//! Off-screen render of the `/integrations` listing, on the plain rung
//! (`toolshot_tests` holds the harness).
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew integrations_shot -- --ignored --nocapture`
use crate::toolshot_tests::{intact, tools_shot};

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn integrations_shot_as_a_tile_and_empty() {
    use crate::integrview::tests::weather;
    use crew_plugin::integration::Auth;
    let _g = crate::app::theme_test_guard();
    let ints = vec![
        weather(Auth::Bearer {
            env: "WEATHER_TOKEN".into(),
        }),
        weather(Auth::None),
    ];
    for (name, text, w) in [
        (
            "integrations-tile",
            crate::integrview::listing(&ints, &|_| false),
            420u32,
        ),
        (
            "integrations-empty",
            crate::integrview::listing(&[], &|_| false),
            640,
        ),
    ] {
        let Some(rows) = tools_shot(name, &text, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        intact(&rows, &text, name);
    }
}
