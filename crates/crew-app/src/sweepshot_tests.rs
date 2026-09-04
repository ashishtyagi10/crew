//! Width sweeps for the surfaces that were only ever shot at one width.
//!
//! `/usage` and the model picker had one shot each (760px and 640px); the
//! `/watching` and `/integrations` listings tied each width to a different
//! content case, so the wrapped detail rows were never seen wide and the
//! empty hints never in the tile they appear in. A layout that branches on
//! width has never been seen until each branch has.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew sweep_shot -- --ignored --nocapture`
use crate::goalshot_tests::dump;
use crate::shotgpu_tests::shot_at;
use crate::toolshot_tests::{intact, tools_shot};

fn skip() {
    eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
}

/// The model picker at a tile and at a whole window: slug, hint and the
/// `needs a key` / dim shapes negotiating one row.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sweep_shot_model_picker_widths() {
    let _g = crate::app::theme_test_guard();
    let items = crate::menushot_tests::models();
    for (name, w) in [("menu-models-narrow", 380u32), ("menu-models-wide", 1100)] {
        let Some(px) = crate::menushot_tests::menu_shot(name, &items, 1, w) else {
            return skip();
        };
        assert!(crate::shotgpu_tests::ink(&px) > 1000, "{name} drew");
    }
}

/// `/usage` as a quarter tile and as the whole window; the rows dumped so a
/// label that lands on the wrong column is a diff, not a squint.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sweep_shot_usage_widths() {
    let _g = crate::app::theme_test_guard();
    let mut hourly = vec![0u64; crate::usageledger::DAYS * crate::usageledger::HOURS];
    for (i, v) in hourly.iter_mut().enumerate() {
        *v = ((i * 37) % 11) as u64 * 900;
    }
    let b = crate::usageledger::Buckets {
        hourly,
        daily_cost: vec![120_000, 340_000, 0, 20_000, 810_000, 430_000, 260_000],
        tok_in: 1_840_000,
        tok_out: 410_000,
        cost_microusd: 1_980_000,
    };
    for (name, w, h) in [("usage-tile", 380u32, 300u32), ("usage-wide", 1100, 560)] {
        let mut dumped = Vec::new();
        let got = shot_at(name, w, h, 13.0, "usage", |cols, rows, aspect| {
            let cells = crate::usagepane::cells(&b, cols, rows);
            dumped = dump(&cells, cols, rows);
            eprintln!("--- {name} {cols}x{rows}");
            for l in &dumped {
                eprintln!("|{l}");
            }
            (cells, crate::usagepane::paint(&b, cols, rows, aspect))
        });
        if got.is_none() {
            return skip();
        }
        let all = dumped.join("\n");
        assert!(
            all.contains("$1.98"),
            "{name}: the total is on the card:\n{all}"
        );
    }
}

/// The two listings with the SAME text at both widths: the wrapped detail
/// rows wide, and the empty hint in a tile.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sweep_shot_listings_both_ways() {
    use crate::integrview::tests::weather;
    use crew_plugin::integration::Auth;
    let _g = crate::app::theme_test_guard();
    let (standing, history) = crate::watchshot_tests::fixture();
    let now = crate::toolshot_tests::NOW;
    let ints = vec![weather(Auth::Bearer {
        env: "WEATHER_TOKEN".into(),
    })];
    let cases = [
        (
            "watching-wide",
            crate::watchview::listing(&standing, &history, now),
            1000u32,
        ),
        (
            "watching-empty-tile",
            crate::watchview::listing(&[], &history, now),
            420,
        ),
        (
            "integrations-wide",
            crate::integrview::listing(&ints, &|_| true),
            1000,
        ),
        (
            "integrations-empty-tile",
            crate::integrview::listing(&[], &|_| false),
            420,
        ),
    ];
    for (name, text, w) in cases {
        let Some(rows) = tools_shot(name, &text, w) else {
            return skip();
        };
        intact(&rows, &text, name);
    }
}
