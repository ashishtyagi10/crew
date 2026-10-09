use super::*;

fn bar<'a>(focused: bool, broadcast: bool) -> Bar<'a> {
    Bar {
        index: Some(1),
        title: "zsh",
        focused,
        broadcast,
        ..Default::default()
    }
}

/// The loud one: while broadcast is on, every pane it reaches wears the
/// broadcast colour — focused or not, so the group reads as a group.
#[test]
fn broadcast_paints_the_whole_frame_on_every_pane_it_reaches() {
    let _g = crate::app::theme_test_guard();
    let cast = crew_theme::theme().broadcast;
    let hue = (10, 20, 30);
    assert_eq!(stroke(&bar(false, true), hue).0, cast);
    assert_eq!(stroke(&bar(true, true), hue).0, cast);
}

/// …and off, nothing changes: the focused card keeps its bright stroke and
/// the rest keep the normal one.
#[test]
fn without_broadcast_the_frame_is_the_one_it_always_was() {
    let _g = crate::app::theme_test_guard();
    let t = crew_theme::theme();
    let hue = (10, 20, 30);
    assert_eq!(
        stroke(&bar(true, false), hue).0,
        crate::panecardglow::focused_stroke(t)
    );
    assert_eq!(stroke(&bar(false, false), hue).0, t.border_normal);
}

/// The legend keeps saying WHICH pane this is in either mode — the frame
/// carries the mode, the legend carries the identity.
#[test]
fn the_legend_keeps_the_panes_own_hue() {
    let _g = crate::app::theme_test_guard();
    let hue = (200, 40, 90);
    assert_eq!(stroke(&bar(true, true), hue).1, hue);
    assert_eq!(stroke(&bar(true, false), hue).1, hue);
    let dim = stroke(&bar(false, true), hue).1;
    assert_ne!(dim, hue, "an unfocused legend recedes");
    assert_eq!(dim, stroke(&bar(false, false), hue).1);
}

/// On see-through glass the legend stands on the desktop: a mid-tone hue
/// read 2.3:1 on light glass over a dark desktop (2026-10-09, "hard to see
/// title of the panels"). Focused or not, it now reads over any desktop.
#[test]
fn glass_legends_read_over_any_desktop() {
    let _g = crate::app::theme_test_guard();
    let floor = |f: bool| match f {
        true => crew_theme::glasslegend::FOCUSED_LABEL_FLOOR,
        false => crew_theme::glasslegend::LABEL_FLOOR,
    };
    for id in [
        crew_theme::ThemeId::GlassSky,
        crew_theme::ThemeId::GlassDawn,
        crew_theme::ThemeId::GlassNight,
    ] {
        crew_theme::set_theme(id);
        let grounds = crew_theme::glasslegend::grounds(crew_theme::theme());
        for focused in [true, false] {
            let legend = stroke(&bar(focused, false), (55, 82, 113)).1;
            for g in &grounds {
                let got = crew_theme::contrast_ratio(legend, *g);
                assert!(got >= floor(focused), "{id:?} {focused}: {got:.2} on {g:?}");
            }
        }
    }
}
