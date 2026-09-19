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
pub(crate) fn shot(
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
fn chart_shot_sys_dials() {
    let _g = crate::app::theme_test_guard();
    let stats = crate::stats::Stats {
        cpu: 0.34,
        mem: 0.78,
        disk: 0.94,
        ..Default::default()
    };
    let px = shot("dials", "SYSTEM", |cols, _rows, aspect| {
        (
            crate::sysdials::NAV.cells(stats, cols, 0),
            crate::sysdials::NAV.paint(stats, cols, 0, aspect),
        )
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    let ink = ink(&px);
    assert!(ink > 2000, "the dials drew something: {ink} ink pixels");
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
            crate::net::net_cells(842_000, 121_000, 842_000, cols),
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
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    // On a light page as well as a dark one: the trough is the ramp pulled
    // toward the page and then drawn at 55% alpha, which is a different
    // reading on every page — and the light ones were where it stopped being
    // a groove at all (`summarymeter::TROUGH_FLOOR`).
    for (name, id) in [
        ("meters", crew_theme::ThemeId::PaperDark),
        ("meters-light", crew_theme::ThemeId::PaperLight),
        ("meters-crt", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        if meters_shot(name).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
}

/// The footer's second line, as it is placed: countdowns, then the two
/// reserved meter runs with their labels.
fn meters_shot(name: &str) -> Option<Vec<u8>> {
    let px = shot(name, "crew", |_cols, _rows, aspect| {
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
    })?;
    let ink = ink(&px);
    assert!(ink > 500, "{name}: the meters drew something: {ink} pixels");
    Some(px)
}
