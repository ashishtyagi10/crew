//! `CREW_SHOT_FONT=<family>`: shoot in an installed face at the app's default
//! weight — how a new font is looked at before crew offers it. And
//! `CREW_SHOT_GLASS=<level>`: shoot at a user's own glass level (the default
//! is the app's, medium).
use crew_render::CellGrid;

pub fn apply(grid: &mut CellGrid) {
    if let Ok(family) = std::env::var("CREW_SHOT_FONT") {
        crate::glyphs::set_family(Some(&family));
        grid.set_font_family(Some(family));
        grid.set_font_weight(Some(500));
    }
    let level = std::env::var("CREW_SHOT_GLASS").ok();
    if let Some(l) = level.and_then(|s| crew_theme::GlassLevel::parse(&s)) {
        grid.set_glass(l);
    }
}
