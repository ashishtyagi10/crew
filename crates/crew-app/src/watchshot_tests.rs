//! Off-screen render of the `/watching` listing — the standing intents, on
//! the same plain rung `/tools` draws on (`toolshot_tests` holds the harness).
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew watching_shot -- --ignored --nocapture`
use crate::toolshot_tests::{intact, tools_shot, NOW};

/// The other listing crew writes itself: `/watching`, on the same rung. A
/// task is free text and a tile is narrow, so its rows may wrap — on words.
/// Three standing intents and one firing history, for this shot and the
/// width sweep in `sweepshot_tests`.
pub(crate) fn fixture() -> (
    Vec<crate::daemon::intent::Intent>,
    std::collections::BTreeMap<String, crate::daemon::intenthistory::Fired>,
) {
    use crate::daemon::intent::{Intent, Repeat};
    let intent = |id: &str, text: &str, to: &str, fire_in: u64, repeat: Repeat| Intent {
        id: id.into(),
        text: text.into(),
        to: to.into(),
        fire_ms: NOW + fire_in,
        repeat,
        created_ms: NOW - 3 * 86_400_000,
        anchor_ms: None,
    };
    let standing = vec![
        intent(
            "w1",
            "brief me on the calendar",
            "telegram:42",
            8_040_000,
            Repeat::Every { secs: 86_400 },
        ),
        intent(
            "w2",
            "chase the invoice if it has not landed",
            "",
            5 * 86_400_000,
            Repeat::Once,
        ),
        intent(
            "w3",
            "check the deploy",
            "telegram:42",
            30 * 60_000,
            Repeat::Every { secs: 1_800 },
        ),
    ];
    let mut history = std::collections::BTreeMap::new();
    history.insert(
        "w1".to_string(),
        crate::daemon::intenthistory::Fired {
            count: 40,
            last_ms: NOW - 16 * 3_600_000,
            missed: 3,
        },
    );
    (standing, history)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn watching_shot_standing_and_empty() {
    let _g = crate::app::theme_test_guard();
    let (standing, history) = fixture();
    for (name, text, w) in [
        (
            "watching-tile",
            crate::watchview::listing(&standing, &history, NOW),
            420u32,
        ),
        (
            "watching-empty",
            crate::watchview::listing(&[], &history, NOW),
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
