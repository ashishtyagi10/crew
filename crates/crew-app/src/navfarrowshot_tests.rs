//! The Far list's marks, rendered: a hovered row's `×`, a gone folder, the Far pane's folder.
use super::tests::{done, folders};
use super::*;

/// The Far list with a hovered row (its `×` lit), a gone row and the Far pane's folder.
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew navfar_row_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navfar_row_shot() {
    let _g = crate::app::theme_test_guard();
    let (base, dirs) = folders("shot", &["photos", "gonedir", "projects", "notes"]);
    std::fs::remove_dir_all(&dirs[1]).unwrap();
    for id in [
        crew_theme::ThemeId::TokyoNight,
        crew_theme::ThemeId::Porcelain,
    ] {
        let name = format!("navfar-rows-{id:?}").to_lowercase();
        let prep = |app: &mut CrewApp| {
            app.config.show_nav = true;
            app.config.nav_collapsed = false;
            app.config.nav_section = "far".into();
            crate::fargone::reset();
            crate::fargone::refresh(0, true);
            let k = crate::farrecent::list().iter().position(|d| d == &dirs[2]);
            app.far_open_item(k);
            app.input.focused = false;
            // The pointer on the newest row's × (content row 1), in the frame's own cells:
            // the panel's interior starts a cell past its rect, as `far_x_at_cursor` reads it.
            let (cw, ch, ..) = app.frame_geometry().unwrap();
            let (panel, _, _) = app.nav_hit_geometry().unwrap();
            let bar = f32::from(crate::navopen::BAR_TAKES) * cw;
            let (card, _) = crate::layout::card_inner_cells(panel.w + bar, panel.h, cw, ch);
            let x = x_col(card - crate::navopen::BAR_TAKES);
            app.cursor = (panel.x + (f32::from(x) + 1.5) * cw, panel.y + 2.5 * ch);
        };
        let opacity = crate::tubesheer::sheer(id.theme());
        crate::crtglassshot_tests::window_with(&name, id, opacity, &["/far"], &prep);
    }
    done(&base);
}
