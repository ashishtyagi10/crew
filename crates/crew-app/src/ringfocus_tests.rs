//! The focused ring on a SHEER window, where every glass theme runs by
//! default. `modernring`'s hierarchy test measures an opaque window, where a
//! quiet stroke sits at 2:1, and only the ring's brightest cell; on glass the
//! quiet strokes are lifted to 4.5:1 (`glassborder::NORMAL_FLOOR`) and five
//! palettes' rings drew focus no brighter than the cards round them.
use crew_render::CellView;

use crate::panecard::{is_frame_glyph, pane_card, Bar};

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
        // No focus brackets: they are the accent's, not the ring's.
        focus_t: 0.0,
        assemble_t: 1.0,
        git: None,
        ticks: &[],
        hits: &[],
        progress: None,
        elapsed: None,
        pinned: false,
        at_cmd: None,
        fail_rows: &[],
        cmd_rows: &[],
        err_rows: &[],
        unread: 0,
        doc: crate::panehome::Doc::No,
    }
}

/// Every stroke cell's contrast against the page.
fn contrasts(v: &[CellView], page: (u8, u8, u8)) -> Vec<f32> {
    v.iter()
        .filter(|c| is_frame_glyph(c.c))
        .map(|c| crew_theme::contrast_ratio(c.fg, page))
        .collect()
}

/// Puts the window back to opaque however the test ends.
struct Opaque;
impl Drop for Opaque {
    fn drop(&mut self) {
        crew_theme::glassborder::set_sheer(false);
    }
}

/// On glass, the DIMMEST cell of the focused ring stands a clear step over
/// the BRIGHTEST cell of a quiet card, on every palette that draws a ring.
#[test]
fn on_glass_the_whole_focused_ring_outshines_every_quiet_stroke() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let _o = Opaque;
    crew_theme::glassborder::set_sheer(true);
    let mut rings = 0;
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        let t = crew_theme::theme();
        if t.modern.is_none() {
            continue;
        }
        rings += 1;
        let mut lit = pane_card(38, 10, &bar(true));
        crate::modernring::ring(&mut lit, 40, 12, false, 1.0, 0);
        let dimmest = contrasts(&lit, t.page_bg)
            .into_iter()
            .fold(f32::MAX, f32::min);
        let quiet = contrasts(&pane_card(38, 10, &bar(false)), t.page_bg)
            .into_iter()
            .fold(0.0, f32::max);
        let floor = crew_theme::glassborder::FOCUSED_FLOOR;
        assert!(
            dimmest >= floor - 0.05 && dimmest >= quiet * 1.25,
            "{}: the ring dips to {dimmest:.2}:1 (floor {floor}), a quiet card reaches {quiet:.2}:1",
            id.as_str()
        );
    }
    assert!(rings >= 10, "only {rings} palettes drew a ring");
}
