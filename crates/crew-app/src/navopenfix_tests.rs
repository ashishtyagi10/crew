//! What the activity bar's tests stand on: an app with a geometry, and finding what its frame
//! drew. Split from `navopen_tests` for the line cap.
use crate::app::CrewApp;
use crate::navbar::BarHit;
use crate::navsection::NavSection;
use crate::pane::PaneContent;

pub(crate) const CELL: (f32, f32) = (8.0, 17.0);

pub(crate) fn app(h: f32, collapsed: bool, panes: usize) -> CrewApp {
    let mut app = CrewApp {
        geo_override: Some((CELL.0, CELL.1, 1280.0, h, 1.0)),
        ..Default::default()
    };
    app.config.show_nav = true;
    app.config.nav_collapsed = collapsed;
    for i in 0..panes {
        let f = std::env::temp_dir().join(format!("navbar-{i}.txt"));
        let v = crate::viewpane::ViewPane::open(f);
        app.panes
            .push(crate::glassworkshot_tests::pane(PaneContent::View(v), None));
    }
    app.input.focused = false;
    app.reconcile_grid();
    app
}

/// Every place `glyph` is drawn, as the pixel at the middle of its cell.
pub(crate) fn drawn(app: &mut CrewApp, glyph: char) -> Vec<(f32, f32)> {
    let scenes = app.build_frame();
    scenes
        .iter()
        .flat_map(|s| {
            // Only what is whole on screen: a cell past the scene's foot is clipped, not drawn.
            let whole = move |c: &&crew_render::CellView| f32::from(c.row + 1) * CELL.1 <= s.h;
            s.cells
                .iter()
                .filter(|c| c.c == glyph)
                .filter(whole)
                .map(move |c| {
                    let x = s.x + (f32::from(c.col) + 0.5) * CELL.0;
                    let y = s.y + (f32::from(c.row) + 0.5) * CELL.1;
                    (x, y)
                })
        })
        .collect()
}

pub(crate) fn hits() -> Vec<BarHit> {
    let mut v: Vec<BarHit> = NavSection::ALL.into_iter().map(BarHit::Section).collect();
    v.push(BarHit::Settings);
    v
}

/// Where `hit`'s icon is DRAWN, as the pixel at its centre: the nav's interior scene must carry
/// paint around the centre `navicons` names for it, wholly inside the scene. `None` when the bar
/// had no room to draw it.
pub(crate) fn drawn_icon(a: &mut CrewApp, hit: BarHit) -> Option<(f32, f32)> {
    let scenes = a.build_frame();
    let s = scenes
        .iter()
        .find(|s| s.x < 30.0 && s.h > 60.0 && !s.paint.is_empty())?;
    let inked_at = |cx: f32, cy: f32| {
        let near = |p: &&crew_render::Paint| {
            let (px, py) = (p.x + p.w * 0.5, p.y + p.h * 0.5);
            (px - cx).abs() < 1.4 && (py - cy).abs() < 0.75
        };
        // Inked there, and every stroke of it inside the scene: a clipped icon is not drawn.
        let ink: Vec<_> = s.paint.iter().filter(near).collect();
        ink.len() > 4 && ink.iter().all(|p| (p.y + p.h) * CELL.1 <= s.h + 0.5)
    };
    // A section's row does not depend on the bar's height; the gear's is the last row, found by
    // looking for it on the bottom rows rather than by trusting a row count.
    let shown = (s.h / CELL.1).floor() as u16;
    let (cx, cy) = match hit {
        // The gear is the only paint in the bar's columns below the section icons: its centre is
        // the middle of that paint (on the rail, under the readings' meters too — so only the
        // icon column's middle, where no meter reaches, is taken).
        BarHit::Settings => {
            let mid = f32::from(crate::navbar::BAR_COLS) * 0.5;
            let below = f32::from(crate::navbar::HEAD_ROWS);
            let ys: Vec<f32> = s
                .paint
                .iter()
                .filter(|p| p.y > below && (p.x + p.w * 0.5 - mid).abs() < 1.4)
                .flat_map(|p| [p.y, p.y + p.h])
                .collect();
            // The lowest ink, and only what lies within an icon's height of it: the gear, not
            // the rail's meters above it.
            let hi = ys.iter().copied().fold(f32::MIN, f32::max);
            let lo = ys
                .iter()
                .copied()
                .filter(|&y| y > hi - 1.3)
                .fold(f32::MAX, f32::min);
            let _ = shown;
            (lo < hi).then_some((mid, (lo + hi) * 0.5))?
        }
        _ => crate::navicons::centre(hit, shown)?,
    };
    inked_at(cx, cy).then_some((s.x + cx * CELL.0, s.y + cy * CELL.1))
}
