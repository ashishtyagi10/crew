use super::*;
use crate::pane::spawn_pane;
use crew_term::GridSize;

fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

/// Down the list is +1, up is -1, and a tab that is not there is no
/// direction at all.
#[test]
fn direction_follows_nav_order() {
    let mut tabs = Tabs::default();
    let second = tabs.push();
    let third = tabs.push();
    assert_eq!(tabs.pos(third), Some(2));
    assert_eq!(direction(&tabs, second), 1);
    tabs.active = third;
    assert_eq!(direction(&tabs, 1), -1);
    assert_eq!(direction(&tabs, 99), 0);
}

/// A card starts a short way toward the side it comes from, same size.
#[test]
fn the_start_is_a_short_vertical_travel() {
    let tile = r(10.0, 100.0, 400.0, 500.0);
    assert_eq!(start(tile, 1), r(10.0, 140.0, 400.0, 500.0));
    assert_eq!(start(tile, -1), r(10.0, 60.0, 400.0, 500.0));
}

fn shell() -> crate::pane::Pane {
    let grid = GridSize { cols: 20, rows: 5 };
    let mut p = spawn_pane("sh", "sh", grid, Some(&std::env::temp_dir())).unwrap();
    p.rect = crate::spawn::PLACEHOLDER_RECT;
    p
}

/// Through `build_frame`: going back up to the first tab, its card is drawn
/// above its tile first, then settles onto it.
#[test]
fn going_up_the_list_drops_the_tab_in_from_above() {
    let _g = crate::appearanceguard::theme_test_guard();
    crate::motion::set_level(crate::motion::MotionLevel::Full);
    let mut app = crate::app::CrewApp {
        geo_override: Some((8.0, 17.0, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    app.panes.push(shell());
    app.focused = 0;
    let settle = |app: &mut crate::app::CrewApp| {
        for _ in 0..200 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            app.build_frame();
            if !app.glide_active {
                break;
            }
        }
    };
    settle(&mut app);
    let tile = app.panes[0].rect;
    app.own_tab();
    app.panes.push(shell());
    app.focused = 1;
    settle(&mut app);
    let first_tab = app.tabs.id_at(1).unwrap();
    app.activate_tab(first_tab);
    std::thread::sleep(std::time::Duration::from_millis(150));
    app.build_frame();
    let drawn = app.panes[0].rect;
    assert!(app.glide_active, "the tab's card is travelling");
    // Most of the 8% travel is still ahead after the first frame.
    let travel = tile.h * TRAVEL;
    assert!(
        drawn.y < tile.y - 0.6 * travel,
        "starts above its tile: {drawn:?} vs {tile:?}"
    );
    settle(&mut app);
    assert_eq!(app.panes[0].rect.y, tile.y, "and lands on it");
}
