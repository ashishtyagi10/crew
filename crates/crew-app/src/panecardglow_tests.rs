use std::hash::{DefaultHasher, Hash, Hasher};

use super::*;

fn bar(focused: bool) -> Bar<'static> {
    Bar {
        index: Some(2),
        title: "shell",
        focused,
        scroll: 0,
        total: 0,
        activity: false,
        bell: false,
        broadcast: false,
        min_btn: false,
        focus_t: 1.0,
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        unread: 0,
        doc: false,
    }
}

fn far_pane() -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: crate::pane::PaneContent::Far(crate::farpane::FarPane::new(std::env::temp_dir())),
        grid: crew_term::GridSize { cols: 38, rows: 10 },
        rect: crate::layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 192.0,
        },
        label: None,
        name: Some("shell".into()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

fn hash_cells(v: &[CellView]) -> u64 {
    let mut h = DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

fn fg_at(v: &[CellView], col: u16, row: u16) -> (u8, u8, u8) {
    v.iter()
        .find(|c| c.col == col && c.row == row)
        .expect("frame cell exists")
        .fg
}

/// The nodes: a settled (ignite_t = 1.0, idle) focused CRT frame carries hot
/// corners and — crucially for the idle-static invariant — edges that are
/// EXACTLY `border_focused` again, bit for bit.
#[test]
fn settled_crt_frame_has_hot_corners_and_exact_edges() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let base = crew_theme::theme().border_focused;
    let mut v = pane_card(38, 10, &bar(true));
    trace(&mut v, 40, 12, false, 1.0, 0);
    let hot = corner_hot(base);
    assert_ne!(hot, base, "the node colour must actually differ");
    for (col, row) in [(0, 0), (39, 0), (0, 11), (39, 11)] {
        assert_eq!(fg_at(&v, col, row), hot, "corner ({col},{row}) not a node");
    }
    // An edge midpoint (past the focus brackets) rests at border_focused.
    assert_eq!(
        fg_at(&v, 0, 5),
        base,
        "idle edge must return to rest exactly"
    );
}

/// Ignition: at t = 0 the whole stroke starts at the node colour; the legend
/// (and anything else riding the border) keeps its own colour throughout.
#[test]
fn ignition_starts_the_whole_frame_hot() {
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
    let base = crew_theme::theme().border_focused;
    let mut v = pane_card(38, 10, &bar(true));
    trace(&mut v, 40, 12, false, 0.0, 0);
    assert_eq!(fg_at(&v, 0, 5), corner_hot(base), "frame should ignite hot");
    let hue = crate::chatroster::agent_color("shell");
    assert!(
        v.iter().any(|c| c.c == 's' && c.row == 0 && c.fg == hue),
        "the legend must keep its signature hue through ignition"
    );
}

/// Breathing is a pure colour ramp: zero breath is `border_focused` exactly,
/// peak breath lifts toward the hot pole but never as far as the nodes.
#[test]
fn breathing_lifts_the_edge_but_never_past_the_nodes() {
    let base = (0u8, 255u8, 120u8);
    let hot = corner_hot(base);
    assert_eq!(edge_color(base, hot, 1.0, 0.0), base);
    let peak = edge_color(base, hot, 1.0, BREATH_AMP);
    assert_eq!(peak, crate::anim::lerp_rgb(base, HOT_POLE, BREATH_AMP));
    assert_ne!(peak, base, "peak breath must move the pixels");
    for (p, h) in [(peak.0, hot.0), (peak.1, hot.1), (peak.2, hot.2)] {
        assert!(p <= h, "breathing must stay below the corner nodes");
    }
}

/// The guard rails: paper themes get ZERO frame changes, and an unfocused CRT
/// frame stays a quiet trace (no nodes — the hierarchy is the whole point).
#[test]
fn an_unfocused_frame_stays_a_plain_quiet_trace_on_every_theme() {
    let _g = crate::app::theme_test_guard();
    let p = far_pane();
    // Every theme carries a gradient now, so "paper frames are pixel-identical to the plain
    // card" stopped being true — that was the old two-theme modern family. What must still hold
    // is the hierarchy: an UNFOCUSED frame is quiet, whatever the palette. A ring on every pane
    // at once is not a focus cue, it is wallpaper.
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        assert_eq!(
            hash_cells(&pane_card_glowing(&p, &bar(false))),
            hash_cells(&pane_card(38, 10, &bar(false))),
            "{} draws something on an unfocused frame",
            id.as_str()
        );
    }
}
