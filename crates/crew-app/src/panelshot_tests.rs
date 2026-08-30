//! Off-screen render of the three DRAWN panes — `/usage`, `/dash`, `/disk` —
//! at the sizes the auto-grid actually hands a tile, and in the themes the app
//! can be wearing.
//!
//! Each of them already had a shot in `chartshot_tests`: one width, one
//! height, one theme. That is the size a widget is *designed* at, and the
//! width sweeps run over the input bar and `/keys` are what found the things
//! that only go wrong somewhere else on the range — a label flush against its
//! number, a badge invisible for four releases. These panes are pure drawing
//! on an aspect-corrected canvas, which is exactly the kind of thing that
//! survives one size and falls apart at another.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `cargo test -p crew-app --bin crew panel_shot -- --ignored --nocapture`
use crate::shotgpu_tests::{ink, shot_at};

/// A quarter tile on a laptop, a half tile, and the whole window — the same
/// three the chat sweep uses, so a pane can be compared against the pane
/// beside it.
const WIDTHS: [(&str, u32, u32); 3] = [
    ("quarter", 470, 380),
    ("half", 700, 560),
    ("full", 1180, 760),
];

/// A working week in the ledger: office hours on five days, a weekend off, and
/// one late session that cost more than the days around it.
fn week() -> crate::usageledger::Buckets {
    let mut hourly = vec![0u64; crate::usageledger::DAYS * crate::usageledger::HOURS];
    for d in 0..crate::usageledger::DAYS {
        for h in 0..crate::usageledger::HOURS {
            let weekend = d == 2 || d == 3;
            let work = (9..19).contains(&h);
            hourly[d * crate::usageledger::HOURS + h] = match (weekend, work) {
                (true, _) => 0,
                (false, true) => 4_000 + (d * 900 + h * 700) as u64 % 9_000,
                (false, false) => (h as u64 % 5) * 400,
            };
        }
    }
    hourly[5 * crate::usageledger::HOURS + 22] = 26_000;
    crate::usageledger::Buckets {
        hourly,
        daily_cost: vec![120_000, 340_000, 0, 20_000, 810_000, 430_000, 260_000],
        tok_in: 1_840_000,
        tok_out: 410_000,
        cost_microusd: 1_980_000,
    }
}

/// A repo's own shape: `target` dominating everything, then the crates, then
/// the small stuff a `du | sort` would have you reading line by line.
fn repo_disk() -> crate::diskpane::DiskPane {
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
    p
}

/// Shoot every drawn pane at one size, returning `(name, ink)` per pane.
fn sweep_at(suffix: &str, w: u32, h: u32) -> Vec<(String, usize)> {
    let b = week();
    let mut d = crate::dashpane::DashPane::new();
    d.seed_for_test();
    let disk = repo_disk();
    let mut out = Vec::new();
    let mut take = |name: String, px: Option<Vec<u8>>| {
        if let Some(px) = px {
            let n = ink(&px);
            eprintln!("{name}: {n} ink px");
            out.push((name, n));
        }
    };
    take(
        format!("usage-{suffix}"),
        shot_at(
            &format!("usage-{suffix}"),
            w,
            h,
            13.0,
            "usage",
            |c, r, a| {
                (
                    crate::usagepane::cells(&b, c, r),
                    crate::usagepane::paint(&b, c, r, a),
                )
            },
        ),
    );
    take(
        format!("dash-{suffix}"),
        shot_at(&format!("dash-{suffix}"), w, h, 13.0, "dash", |c, r, a| {
            (d.cells(c, r), d.paint(c, r, a))
        }),
    );
    take(
        format!("disk-{suffix}"),
        shot_at(&format!("disk-{suffix}"), w, h, 13.0, "disk", |c, r, a| {
            (disk.cells(c, r), disk.paint(c, r, a))
        }),
    );
    out
}

/// Every drawn pane at every tile size. A pane that draws nothing at one of
/// them is a pane that is blank on somebody's screen — the assertion is only
/// a floor; the PNGs are the point.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn panel_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let mut any = false;
    for (suffix, w, h) in WIDTHS {
        for (name, n) in sweep_at(suffix, w, h) {
            any = true;
            assert!(n > 1_500, "{name} is all but blank: {n} ink pixels");
        }
    }
    if !any {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}

/// The same three panes on a light page and through a green tube. They are
/// drawn from `palette::accent()` and the theme's own roles, and both of those
/// change under the frame.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn panel_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (suffix, id) in [
        ("light", crew_theme::ThemeId::PaperLight),
        ("crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        for (name, n) in sweep_at(suffix, 1180, 760) {
            assert!(n > 1_500, "{name} is all but blank: {n} ink pixels");
        }
    }
}
