//! Off-screen renders of the charts drawn INSIDE a card: the swarm block's
//! timeline and the indicators a pane card wears on its borders.
//!
//! Split from [`crate::panechartshot_tests`] (the whole-pane dashboards) for
//! the line cap — a chart that shares a card with text answers to the text's
//! width, which is a different question from one that owns a pane.
use crew_render::CellView;

use crate::chartshot_tests::shot;
use crate::shotgpu_tests::ink;

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
