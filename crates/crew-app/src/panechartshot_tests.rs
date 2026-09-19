//! Off-screen renders of the drawn PANES — the surfaces whose whole content
//! is a chart: `/usage`, the swarm timeline, the `/disk` treemap, a pane
//! card's indicators, and the dashboard.
//!
//! Split from [`crate::chartshot_tests`] (the widget-sized shots: an area
//! curve, the dials, the net twin, the footer meters) when the meters gained
//! a light page and a tube to be shot on.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew chart_shot -- --ignored`

use crate::chartshot_tests::shot;
use crate::shotgpu_tests::ink;

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_usage_pane() {
    let _g = crate::app::theme_test_guard();
    // A plausible week: work in office hours, a quiet weekend, one long
    // evening session.
    let mut hourly = vec![0u64; crate::usageledger::DAYS * crate::usageledger::HOURS];
    for d in 0..crate::usageledger::DAYS {
        for h in 0..crate::usageledger::HOURS {
            let weekend = d == 2 || d == 3;
            let work = (9..19).contains(&h);
            let v = match (weekend, work) {
                (true, _) => 0,
                (false, true) => 4_000 + (d * 900 + h * 700) as u64 % 9_000,
                (false, false) => (h as u64 % 5) * 400,
            };
            hourly[d * crate::usageledger::HOURS + h] = v;
        }
    }
    hourly[5 * crate::usageledger::HOURS + 22] = 26_000;
    let b = crate::usageledger::Buckets {
        hourly,
        daily_cost: vec![120_000, 340_000, 0, 20_000, 810_000, 430_000, 260_000],
        tok_in: 1_840_000,
        tok_out: 410_000,
        cost_microusd: 1_980_000,
    };
    let px = shot("usage", "usage", |cols, rows, aspect| {
        (
            crate::usagepane::cells(&b, cols, rows),
            crate::usagepane::paint(&b, cols, rows, aspect),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(
        ink > 3000,
        "the usage pane drew something: {ink} ink pixels"
    );
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_disk_treemap() {
    let _g = crate::app::theme_test_guard();
    // A repo's own shape: target dominating, then the crates, then the small
    // stuff that a `du | sort` would have you reading line by line.
    let mut p = crate::diskpane::DiskPane::new(std::env::temp_dir());
    p.set_children_for_test(
        &[
            ("target", 4_509_715_660, true),
            ("crates", 812_000_000, true),
            (".git", 402_000_000, true),
            ("vendor", 121_000_000, true),
            ("docs", 24_000_000, true),
            ("Cargo.lock", 310_000, false),
            ("CHANGELOG.md", 96_000, false),
            ("README.md", 12_000, false),
        ],
        1,
    );
    let px = shot("treemap", "disk", |cols, rows, aspect| {
        (p.cells(cols, rows), p.paint(cols, rows, aspect))
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 5000, "the treemap drew something: {ink} ink pixels");
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_dashboard() {
    let _g = crate::app::theme_test_guard();
    let mut d = crate::dashpane::DashPane::new();
    d.seed_for_test();
    let px = shot("dash", "dash", |cols, rows, aspect| {
        (d.cells(cols, rows), d.paint(cols, rows, aspect))
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 4000, "the dashboard drew something: {ink} ink pixels");
}
