use super::*;
use crate::layout::Rect;
use crate::pane::PaneContent;
use crate::viewpane::ViewPane;

fn pane_with(content: PaneContent) -> Pane {
    Pane {
        content,
        grid: crew_term::GridSize { cols: 40, rows: 12 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 400.0,
            h: 200.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
        born_ms: 0,
    }
}

#[test]
fn a_normal_viewer_is_restorable() {
    let p = pane_with(PaneContent::View(ViewPane::open(
        std::env::temp_dir().join("poll-restorable-test.txt"),
    )));
    assert!(is_restorable_pane(&p));
}

#[test]
fn an_ephemeral_viewer_is_not_restorable() {
    // Fix 4: `/about` and `??` mark their viewer ephemeral because it opens
    // on a synthetic temp file, not something the user asked to view — the
    // regression this predicate exists to fix.
    let mut v = ViewPane::open(std::env::temp_dir().join("poll-ephemeral-test.txt"));
    v.ephemeral = true;
    let p = pane_with(PaneContent::View(v));
    assert!(
        !is_restorable_pane(&p),
        "an ephemeral viewer must not count toward had_restorable"
    );
}
