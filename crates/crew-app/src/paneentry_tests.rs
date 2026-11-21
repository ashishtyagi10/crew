use super::*;
use crate::pane::spawn_pane;
use crew_term::GridSize;

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

fn pane(rect: Rect) -> Pane {
    let grid = GridSize { cols: 20, rows: 5 };
    let mut p = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
    p.rect = rect;
    p
}

const ZERO: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 0.0,
    h: 0.0,
};

/// Cmd+D: the new pane (focused) comes out of the pane focus just left, at
/// the new tile's own size — position travels, size never does.
#[test]
fn a_split_starts_at_its_parent_at_its_own_size() {
    let panes = vec![pane(r(10.0, 20.0, 800.0, 600.0)), pane(ZERO)];
    let placed = [
        (0, r(10.0, 20.0, 400.0, 600.0)),
        (1, r(410.0, 20.0, 400.0, 600.0)),
    ];
    let got = entry(&panes, &placed, 1, 0);
    assert_eq!(got, Some((1, r(10.0, 20.0, 400.0, 600.0))));
}

/// A pane opened in the background comes out of the focused pane.
#[test]
fn a_background_pane_comes_out_of_the_focused_one() {
    let panes = vec![
        pane(r(0.0, 0.0, 300.0, 300.0)),
        pane(r(300.0, 0.0, 300.0, 300.0)),
        pane(ZERO),
    ];
    let placed = [
        (0, r(0.0, 0.0, 200.0, 300.0)),
        (1, r(200.0, 0.0, 200.0, 300.0)),
        (2, r(400.0, 0.0, 200.0, 300.0)),
    ];
    let got = entry(&panes, &placed, 1, 0).map(|(i, f)| (i, f.x));
    assert_eq!(got, Some((2, 300.0)));
}

/// Panes born together, a first pane with nothing to come out of, and a
/// parent that is not on screen (another tab, minimized) all just assemble.
#[test]
fn no_single_parent_means_no_slide() {
    let two_fresh = vec![pane(r(0.0, 0.0, 300.0, 300.0)), pane(ZERO), pane(ZERO)];
    let placed = [
        (0, r(0.0, 0.0, 100.0, 100.0)),
        (1, r(100.0, 0.0, 100.0, 100.0)),
        (2, r(200.0, 0.0, 100.0, 100.0)),
    ];
    assert_eq!(entry(&two_fresh, &placed, 2, 0), None);

    let alone = vec![pane(ZERO)];
    assert_eq!(entry(&alone, &[(0, r(0.0, 0.0, 9.0, 9.0))], 0, 0), None);

    let offscreen = vec![pane(r(0.0, 0.0, 300.0, 300.0)), pane(ZERO)];
    assert_eq!(
        entry(&offscreen, &[(1, r(0.0, 0.0, 300.0, 300.0))], 1, 0),
        None
    );
}

/// Settled panes are not fresh: nothing slides on an ordinary frame.
#[test]
fn a_settled_grid_has_no_entry() {
    let panes = vec![
        pane(r(0.0, 0.0, 300.0, 300.0)),
        pane(r(300.0, 0.0, 300.0, 300.0)),
    ];
    let placed = [(0, panes[0].rect), (1, panes[1].rect)];
    assert_eq!(entry(&panes, &placed, 1, 0), None);
}

/// End to end through `build_frame`: the new card's first frame is drawn at
/// its parent, it travels, and it lands on its tile at the size it started.
#[test]
fn a_split_travels_from_its_parent_to_its_tile() {
    let _g = crate::appearanceguard::theme_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = crate::app::CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.panes.push(pane(ZERO));
    app.focused = 0;
    for _ in 0..3 {
        app.build_frame();
    }
    let parent = app.panes[0].rect;
    assert!(parent.w > 0.0);
    app.panes.push(pane(ZERO));
    app.focused = 1;
    app.build_frame();
    let first = app.panes[1].rect;
    assert!(app.glide_active, "the new card is moving");
    assert!(
        (first.x - parent.x).abs() < 40.0,
        "starts at its parent: {first:?} vs {parent:?}"
    );
    let mut last = first;
    for _ in 0..200 {
        std::thread::sleep(std::time::Duration::from_millis(5));
        app.build_frame();
        last = app.panes[1].rect;
        assert!(
            (last.w - first.w).abs() < 0.5,
            "size never animates: {first:?} → {last:?}"
        );
        if !app.glide_active {
            break;
        }
    }
    assert!(!app.glide_active, "settles");
    assert!(
        last.x > parent.x + 100.0,
        "landed on its own tile: {last:?}"
    );
}
