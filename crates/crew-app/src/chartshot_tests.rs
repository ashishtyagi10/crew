//! Off-screen render of the [`crate::plot`] widgets, so a chart can be *looked
//! at* rather than only asserted on.
//!
//! `#[ignore]`d: needs a real GPU adapter and writes PNGs. Run with
//! `cargo test -p crew-app --bin crew chart_shot -- --ignored --nocapture`;
//! the PNGs land in `$CREW_SHOT_DIR` (default `target/screenshots`).
//!
//! Driving the live GUI needs macOS Accessibility AND Screen Recording, which
//! this session does not have — and a chart is exactly the kind of thing that
//! passes every unit test and still comes out an unreadable smear. Each chart
//! iteration adds its shot here, rendered through the same `CellGrid` the app
//! draws frames with, on a real card over the real paper background.
use crew_render::{CellView, Paint};

use crate::shotgpu_tests::{ink, shot_at};

const W: u32 = 760;
const H: u32 = 560;

/// Render and write `chart-<name>.png`, returning the pixels for assertions.
fn shot(
    name: &str,
    legend: &str,
    content: impl FnOnce(u16, u16, f32) -> (Vec<CellView>, Vec<Paint>),
) -> Option<Vec<u8>> {
    shot_at(&format!("chart-{name}"), W, H, 13.0, legend, content)
}

/// A plausible CPU trace: a slow swell with a spike, the shape the sidebar
/// actually shows.
fn series(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let base = 0.28 + 0.22 * (t * 6.0).sin();
            let spike = if (0.62..0.70).contains(&t) { 0.45 } else { 0.0 };
            (base + spike).clamp(0.02, 0.98)
        })
        .collect()
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_area() {
    let _g = crate::app::theme_test_guard();
    let px = shot("area", "SYSTEM", |cols, rows, aspect| {
        let mut c = crate::plot::Canvas::new(cols, rows, aspect);
        let (w, h) = c.size();
        crate::plot::area::draw(
            &mut c,
            (0.0, 0.0, w, h),
            &series(48),
            crate::palette::accent(),
        );
        (Vec::new(), c.paint())
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    // The chart put ink on the page: some pixel inside the card differs from
    // the page it is drawn on by more than the paper grain.
    let ink = ink(&px);
    assert!(ink > 2000, "the chart drew something: {ink} ink pixels");
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_crew_donut() {
    let _g = crate::app::theme_test_guard();
    let m = crate::crewpie::Mix {
        working: 3,
        waiting: 1,
        idle: 2,
    };
    let mut pulse = crate::spark::History::new(64);
    for (i, v) in (0..48).map(|i| (i, (i % 7) as u64)) {
        let _ = i;
        pulse.push(v);
    }
    let px = shot("donut", "PANES", |cols, rows, aspect| {
        let _ = rows;
        (
            crate::crewpie::cells(&m, cols, 0),
            crate::crewpie::paint(&m, cols, 0, aspect, &pulse),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 2000, "the donut drew something: {ink} ink pixels");
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_sys_rings() {
    let _g = crate::app::theme_test_guard();
    let stats = crate::stats::Stats {
        cpu: 0.34,
        mem: 0.78,
        disk: 0.94,
        ..Default::default()
    };
    let px = shot("rings", "SYSTEM", |cols, _rows, aspect| {
        (
            crate::sysrings::cells(stats, 0),
            crate::sysrings::paint(stats, cols, 0, aspect),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 2000, "the rings drew something: {ink} ink pixels");
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_net_twin() {
    let _g = crate::app::theme_test_guard();
    let mut rx = crate::spark::History::new(64);
    let mut tx = crate::spark::History::new(64);
    for i in 0..48u64 {
        let t = i as f32 / 48.0;
        rx.push((900_000.0 * (0.35 + 0.5 * (t * 7.0).sin().abs())) as u64);
        tx.push((900_000.0 * (0.10 + 0.25 * (t * 4.0).cos().abs())) as u64);
    }
    let px = shot("nettwin", "NET", |cols, _rows, aspect| {
        (
            crate::net::net_cells(842_000, 121_000, cols),
            crate::nettwin::paint(
                &rx,
                &tx,
                cols,
                2,
                aspect,
                crate::net::spark(),
                crate::net::up_color(),
            ),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(
        ink > 2000,
        "the twin chart drew something: {ink} ink pixels"
    );
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn chart_shot_footer_meters() {
    let _g = crate::app::theme_test_guard();
    let px = shot("meters", "crew", |_cols, _rows, aspect| {
        let t = crew_theme::theme();
        let mut cells: Vec<CellView> = Vec::new();
        let mut put = |text: &str, col: u16, row: u16, fg: (u8, u8, u8)| {
            for (i, ch) in text.chars().enumerate() {
                cells.push(CellView {
                    col: col + i as u16,
                    row,
                    c: ch,
                    fg,
                    bg: t.page_bg,
                    ..Default::default()
                });
            }
        };
        // The footer's line 2, as it is placed: countdowns, then the two
        // reserved meter runs with their labels.
        put("5h:2h14m \u{00b7} 7d:5d02h \u{00b7} ", 1, 1, t.ansi[12]);
        put(
            "\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591} 34% (5h) \u{00b7} ",
            25,
            1,
            t.text_muted,
        );
        put(
            "\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591}\u{2591} 71% (ctx)",
            44,
            1,
            t.text_muted,
        );
        let paint = crate::chatsummary::draw_meters(&mut cells, &[0.34, 0.71], aspect);
        (cells, paint)
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 500, "the meters drew something: {ink} ink pixels");
}

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
fn chart_shot_swarm_timeline() {
    let _g = crate::app::theme_test_guard();
    use crate::plot::gantt::Span;
    let t = crew_theme::theme();
    let acc = crate::palette::accent();
    // A swarm that fanned out four ways, then joined: the shape a task list
    // cannot show.
    let spans: Vec<Option<Span>> = vec![
        Some(Span {
            start_ms: 0,
            end_ms: 1_200,
            color: t.ansi[2],
        }),
        Some(Span {
            start_ms: 1_300,
            end_ms: 5_400,
            color: t.ansi[2],
        }),
        Some(Span {
            start_ms: 1_320,
            end_ms: 6_100,
            color: t.ansi[2],
        }),
        Some(Span {
            start_ms: 1_340,
            end_ms: 3_900,
            color: t.ansi[9],
        }),
        Some(Span {
            start_ms: 1_360,
            end_ms: 8_800,
            color: acc,
        }),
        None,
    ];
    let px = shot("timeline", "swarm", |cols, rows, aspect| {
        let mut cells: Vec<CellView> = Vec::new();
        let names = [
            " \u{2713} read the crate",
            " \u{2713} map the render path",
            " \u{2713} map the theme path",
            " \u{2717} bench the atlas",
            " \u{25cf} write the report",
            " \u{25cb} review",
        ];
        let put = |cells: &mut Vec<CellView>, s: &str, row: u16, fg: (u8, u8, u8)| {
            for (i, ch) in s.chars().enumerate() {
                cells.push(CellView {
                    col: i as u16,
                    row,
                    c: ch,
                    fg,
                    bg: t.page_bg,
                    ..Default::default()
                });
            }
        };
        put(&mut cells, " live:1 done:3 failed:1 cost:$0.0421", 0, t.ink);
        for (i, n) in names.iter().enumerate() {
            let fg = match i {
                3 => t.ansi[9],
                4 => acc,
                5 => t.text_muted,
                _ => t.ansi[2],
            };
            put(&mut cells, n, i as u16 + 1, fg);
        }
        cells.extend(crate::swarm::view::timeline_cells(
            cols,
            rows,
            Some((0, 8_800)),
        ));
        let paint =
            crate::swarm::view::timeline_paint(&spans, cols, rows, aspect, (0, 8_800), 8_800);
        (cells, paint)
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 1000, "the timeline drew something: {ink} ink pixels");
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
fn chart_shot_card_indicators() {
    let _g = crate::app::theme_test_guard();
    let px = shot("card", "build \u{00b7} cargo", |cols, rows, aspect| {
        // The card's own frame, with a program reporting 34% and a buffer
        // scrolled a third of the way back.
        let bar = crate::panecard::Bar {
            index: Some(1),
            title: "build \u{00b7} cargo",
            focused: true,
            scroll: 4_000,
            total: 12_000,
            activity: false,
            bell: false,
            broadcast: false,
            min_btn: true,
            assemble_t: 1.0,
            focus_t: 1.0,
            git: None,
            ticks: &[],
            hits: &[],
            progress: Some(crew_term::Progress {
                percent: Some(34),
                alarm: false,
            }),
            elapsed: Some("12s".into()),
            pinned: false,
            at_cmd: None,
            fail_rows: &[],
            cmd_rows: &[],
            err_rows: &[],
            unread: 0,
            doc: false,
        };
        (
            crate::panecard::pane_card(cols.saturating_sub(2), rows.saturating_sub(2), &bar),
            crate::cardpaint::card_paint(cols, rows, &bar, aspect, 0),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(
        ink > 500,
        "the card indicators drew something: {ink} ink pixels"
    );
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
