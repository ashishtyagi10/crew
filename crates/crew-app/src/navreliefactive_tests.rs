use super::*;

fn corner(row: u16, col: u16, c: char, fg: (u8, u8, u8)) -> CellView {
    CellView {
        row,
        col,
        c,
        fg,
        ..Default::default()
    }
}

const ACCENT: (u8, u8, u8) = (67, 72, 190);
const RULE: (u8, u8, u8) = (180, 180, 184);

/// Two tab cards, the second framed in the accent.
fn two() -> (Vec<CellView>, Vec<Box4>) {
    let mut v = Vec::new();
    for (r0, fg) in [(1, RULE), (5, ACCENT)] {
        v.push(corner(r0, 1, '\u{256d}', fg));
        v.push(corner(r0, 20, '\u{256e}', fg));
        v.push(corner(r0 + 2, 1, '\u{2570}', fg));
        v.push(corner(r0 + 2, 20, '\u{256f}', fg));
    }
    let tabs = crate::reliefwell::boxes(&v);
    (v, tabs)
}

#[test]
fn the_card_framed_in_the_accent_is_the_active_tab() {
    let (v, tabs) = two();
    assert_eq!(tabs.len(), 2);
    let on = active(&v, &tabs, ACCENT).expect("an active tab");
    assert_eq!(on.r0, 5);
    assert_eq!(
        active(&v, &tabs, (1, 2, 3)),
        None,
        "no accent, no active tab"
    );
}

#[test]
fn the_active_tab_stands_up_and_the_rest_stay_sunk() {
    assert!(depth(true) > 0.0, "raised");
    assert_eq!(depth(false), DEPTH);
}

/// End to end, on a soft palette with three tabs open: the frame carries
/// exactly one raised tab card, so a change to the active frame's colour in
/// `tabnav` fails here instead of quietly flattening the nav.
#[test]
fn the_frame_raises_exactly_one_tab_card() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::Porcelain);
    crate::palette::set_accent(crew_theme::theme().accent_default);
    let mut app = crate::app::CrewApp {
        geo_override: Some((8.0, 16.0, 1280.0, 720.0, 1.0)),
        ..Default::default()
    };
    for cmd in ["/dash", "/settings", "/todo", "/far"] {
        app.submit_input(cmd.to_string());
    }
    app.config.show_nav = true;
    app.config.nav_collapsed = false;
    crate::tabshot_tests::three_tabs(&mut app);
    // The settings form's keys share the lift; the nav is the left column.
    let in_nav = |s: &&crew_render::PaneScene| s.x + s.w < 260.0;
    let frame = app.build_frame();
    let nav: Vec<_> = frame.iter().filter(in_nav).collect();
    let raised: Vec<f32> = nav
        .iter()
        .filter(|s| s.lift == ACTIVE)
        .map(|s| s.y)
        .collect();
    let sunk: Vec<f32> = nav
        .iter()
        .filter(|s| s.lift == DEPTH)
        .map(|s| s.y)
        .collect();
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert_eq!(raised.len(), 1, "one tab card stands up");
    // The active tab is the first, `build`: above every sunk tab card.
    assert_eq!(sunk.len(), 2, "the other two stay sunk");
    assert!(sunk.iter().all(|&y| y > raised[0]), "{raised:?} {sunk:?}");
}
