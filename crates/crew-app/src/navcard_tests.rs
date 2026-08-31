use crate::app::CrewApp;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{Pane, PaneContent};
use crew_term::GridSize;

#[test]
fn title_max_cols_is_exactly_titled_cards_truncation_budget() {
    let rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 200.0,
        h: 100.0,
    };
    let (cw, ch) = (8.0f32, 16.0f32);
    let max = super::title_max_cols(rect, cw, ch);
    let (icols, irows) = crate::layout::card_inner_cells(rect.w, rect.h, cw, ch);

    // A title exactly `max` chars long survives untruncated.
    let fits: String = "x".repeat(max);
    let cells =
        crate::boxdraw::titled_card(icols + 2, irows + 2, &fits, (0, 0, 0), (0, 0, 0), (0, 0, 0));
    assert_eq!(cells.iter().filter(|c| c.c == 'x').count(), max);

    // One char more than `max` gets ellipsized back down to `max` total
    // columns: `max - 1` kept chars plus the `…` marker.
    let overflows: String = "x".repeat(max + 1);
    let cells2 = crate::boxdraw::titled_card(
        icols + 2,
        irows + 2,
        &overflows,
        (0, 0, 0),
        (0, 0, 0),
        (0, 0, 0),
    );
    assert_eq!(cells2.iter().filter(|c| c.c == 'x').count(), max - 1);
    assert_eq!(cells2.iter().filter(|c| c.c == '…').count(), 1);
}

fn far_pane(name: &str) -> Pane {
    Pane {
        glide: crate::glide::Glide::default(),
        content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: Some(name.to_string()),
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: crate::anim::now_ms(),
    }
}

#[test]
fn zoom_marks_every_covered_pane_restorable() {
    let mut app = CrewApp::default();
    for n in ["a", "b", "c"] {
        app.panes.push(far_pane(n));
    }
    app.focused = 1;
    app.zoomed = true;
    let rows = app.pane_rows();
    assert!(!rows[1].minimized, "the zoomed pane is on screen");
    assert!(
        rows[0].minimized && rows[2].minimized,
        "panes covered by the zoom get the [+] marker"
    );
}

#[test]
fn attention_reaches_the_pane_row_as_a_glyph() {
    let mut app = CrewApp::default();
    for n in ["a", "b"] {
        app.panes.push(far_pane(n));
    }
    crate::attention::raise(
        &mut app.panes[1],
        crate::notify::NotifyKind::Bell,
        crate::anim::now_ms(),
    );
    let rows = app.pane_rows();
    assert_eq!(rows[0].attention, None);
    // Fresh marker: bell glyph, blink phase starts visible.
    assert_eq!(rows[1].attention, Some(('!', true)));
}

#[test]
fn grid_marks_only_nav_hidden_panes_restorable() {
    let mut app = CrewApp::default();
    for n in ["a", "b", "c"] {
        app.panes.push(far_pane(n));
    }
    app.panes[2].hidden = true;
    let rows = app.pane_rows();
    assert!(!rows[0].minimized && !rows[1].minimized);
    assert!(rows[2].minimized);
}
