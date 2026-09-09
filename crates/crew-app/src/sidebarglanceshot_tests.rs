//! Off-screen render of the nav with the glance cards in the LOG's slot.
//! Its own file: `sidebarshot_tests.rs` is over the line cap.
//! `cargo test -p crew-app --bin crew sidebar_shot_glance -- --ignored --nocapture`
use crate::shotgpu_tests::shot_at;
use crate::statspane::StatsPane;

const H: u32 = 1000;

fn cpu_trace() -> Vec<u64> {
    (0..64)
        .map(|i| {
            let t = i as f32 / 64.0;
            (11.0 + 6.0 * (t * 9.0).sin()).clamp(2.0, 99.0) as u64
        })
        .collect()
}

fn pane(index: usize, title: &str, focused: bool, busy: bool) -> crate::panelist::PaneRow {
    crate::panelist::PaneRow {
        index,
        title: title.into(),
        focused,
        activity: false,
        minimized: false,
        attention: None,
        busy,
        unread: 0,
        hovered: false,
    }
}

/// The glance cards in the LOG's slot — SERVING with two live meters,
/// WAITING ON YOU with a blocked shell, a plan and a running task — and
/// the weather strip under the clock: the column as v0.21.64 docks it.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn sidebar_shot_glance() {
    use crate::navglance::{Glance, Serving, Signal};
    use crate::usageledger::{WindowStat, Windows};
    let _g = crate::app::theme_test_guard();
    let mut sp = StatsPane::new();
    sp.refresh(std::path::Path::new("."));
    sp.set_git(Some(crate::git::GitInfo {
        branch: "main".into(),
        changed: 9,
        ahead: 1,
        behind: 0,
    }));
    sp.seed_history(&cpu_trace());
    let panes = vec![
        pane(1, "smith", true, true),
        pane(2, "cargo watch", false, true),
        pane(3, "zsh", false, false),
    ];
    let glance = Glance {
        serving: Serving {
            provider: Some("claude-code".into()),
            model: Some("claude-sonnet-5".into()),
            windows: Windows {
                five_h: Some(WindowStat {
                    left_ms: 2 * 3_600_000 + 10 * 60_000,
                    spent: 42,
                    budget: 100,
                }),
                seven_d: Some(WindowStat {
                    left_ms: 3 * 86_400_000 + 4 * 3_600_000,
                    spent: 15,
                    budget: 100,
                }),
            },
        },
        waiting: crate::navglance::rows_from(vec![
            (
                0,
                "smith".to_string(),
                Signal {
                    blocked: false,
                    plan: true,
                    running: 2,
                },
            ),
            (
                2,
                "zsh".to_string(),
                Signal {
                    blocked: true,
                    plan: false,
                    running: 0,
                },
            ),
        ]),
    };
    let strip = "\u{2600} 24\u{00b0} \u{2191}27 \u{2193}18 \u{2602}10%";
    for (name, w) in [
        ("sidebar-glance-narrow", 160 + 24),
        ("sidebar-glance", 210 + 24),
    ] {
        let px = shot_at(
            name,
            w,
            H,
            13.0,
            concat!("crew v", env!("CARGO_PKG_VERSION")),
            |cols, rows, aspect| {
                (
                    sp.cells(cols, rows, &panes, &[], 0, Some(&glance), Some(strip)),
                    sp.chart_paint(cols, rows, aspect, Some(&glance), panes.len()),
                )
            },
        );
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 4000, "{name} drew");
    }
}
