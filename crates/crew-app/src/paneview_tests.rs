use super::*;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crew_term::GridSize;

/// The regression this file exists to prevent: the Glass setting renders
/// nothing unless a scene the APP builds asks for the sheet. Gating it on
/// `bordered` — which every real scene sets `false` — shipped a glass
/// feature that drew a card only in its own headless harness. Assert
/// against `build_scenes`, not a hand-built `PaneScene`.
#[test]
fn every_pane_gets_exactly_one_glass_card() {
    let scenes = build_scenes(
        &[test_pane(), test_pane()],
        Some(0),
        false,
        None,
        None,
        1.0,
        10.0,
        16.0,
        &Default::default(),
        &[],
    );
    let cards: Vec<_> = scenes.iter().filter(|s| s.glass).collect();
    assert_eq!(cards.len(), 2, "one frosted sheet per pane");
    // The sheet has to cover the whole pane, not the cell-inset content —
    // a sheet drawn on the content scene would leave the frame unfrosted.
    for c in cards {
        assert_eq!((c.w, c.h), (820.0, 416.0), "sheet must span the pane rect");
    }
}

/// The card scene is what a sheer window solidifies (crew-render's
/// `focused_card_rect` looks for `glass && focused && !overlay`), so
/// exactly one card per frame may carry focus — and it must be the FOCUSED
/// pane's, spanning its whole rect. Focus living only on the cell-inset
/// content scene would leave the frame see-through around solid content.
#[test]
fn exactly_one_card_carries_focus() {
    let scenes = build_scenes(
        &[test_pane(), test_pane()],
        Some(1),
        false,
        None,
        None,
        1.0,
        10.0,
        16.0,
        &Default::default(),
        &[],
    );
    let focused: Vec<_> = scenes
        .iter()
        .filter(|s| s.glass && s.focused && !s.overlay)
        .collect();
    assert_eq!(focused.len(), 1, "one card holds focus");
    assert_eq!(
        (focused[0].w, focused[0].h),
        (820.0, 416.0),
        "the focused card spans the pane rect"
    );
}

fn test_pane() -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 820.0,
            h: 416.0,
        },
        label: None,
        name: Some("md".into()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    }
}

#[test]
fn zoomed_scenes_carry_the_minimize_button() {
    let pane = Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 820.0,
            h: 416.0,
        },
        label: None,
        name: Some("md".into()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    };
    let scenes = build_scenes(
        &[pane],
        Some(0),
        false,
        None,
        None,
        1.0,
        10.0,
        16.0,
        &Default::default(),
        &[],
    );
    // scenes[1] is the border card; the [-][x] buttons sit at card columns
    // cols-8..=cols-6 and cols-5..=cols-3 on row 0 (cols = grid cols + 2 border cells).
    let cols = 80 + 2;
    let border = &scenes[1].cells;
    let at = |col: u16| {
        border
            .iter()
            .find(|c| c.row == 0 && c.col == col)
            .map(|c| c.c)
    };
    // The [-] minimize button
    assert_eq!(at(cols - 8), Some('['));
    assert_eq!(at(cols - 7), Some('-'));
    assert_eq!(at(cols - 6), Some(']'));
    // The [x] close button
    assert_eq!(at(cols - 5), Some('['));
    assert_eq!(at(cols - 4), Some('x'));
    assert_eq!(at(cols - 3), Some(']'));
}
