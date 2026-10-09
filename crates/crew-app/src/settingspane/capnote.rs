//! What the theme does to Opacity %, said inside its box.
//!
//! A glass theme sets the window's opacity itself (a quarter: the desktop is
//! its background), and the setting's own floor (35%) sits above that — so on
//! glass the field read `100` while the window plainly showed the desktop,
//! and no value typed into it changed a thing (2026-10-09). A tube caps it
//! too (84%), though there a lower setting still wins. So while the live
//! theme overrides the setting, the box says what the window actually is,
//! the way the accent's box reads out its contrast: a quiet note at the right
//! end, apart from the value being edited.
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;

use super::{form, Field, SettingsPane};

/// The note for `f`, when the live theme overrides what the draft asks for.
pub(super) fn note(p: &SettingsPane, f: Field) -> Option<String> {
    if f != Field::WindowOpacity {
        return inert(f);
    }
    let (who, got) = crate::tubesheer::overridden(p.draft.window_opacity, crew_theme::theme())?;
    Some(format!("{who} sets {}", (got * 100.0).round() as u32))
}

/// Draw [`note`] at the right end of the box `r` holding `value`, when it
/// fits beside the value with a column of air either side.
pub(super) fn draw(buf: &mut Buffer, r: Rect, p: &SettingsPane, f: Field, value: &str) {
    let Some(note) = note(p, f) else { return };
    let w = note.chars().count() as u16;
    if r.height < 2 || r.width < value.chars().count() as u16 + w + 5 {
        return;
    }
    let x = r.x + r.width - 2 - w;
    buf.set_line(
        x,
        r.y + 1,
        &Line::styled(note, Style::new().fg(form::dim())),
        w,
    );
}

/// Said of a canvas setting the live theme leaves nothing to act on. Glass
/// and the tubes paint no paper and no wallpaper of their own — the desktop,
/// or the phosphor, is the background — so Grain, Paper texture and Drifting
/// background read live there and changed nothing (2026-10-09 survey).
fn inert(f: Field) -> Option<String> {
    let t = crew_theme::theme();
    let who = match (t.liquid.is_some(), t.is_tube()) {
        (true, _) => "glass",
        (_, true) => "a tube",
        _ => "this theme",
    };
    match f {
        Field::PaperGrain | Field::PaperTexture if t.grain == 0.0 => {
            Some(format!("no grain on {who}"))
        }
        Field::AmbientDrift if t.modern.is_none_or(|m| m.wash == 0.0) => {
            Some(format!("no wallpaper on {who}"))
        }
        _ => None,
    }
}

/// [`form::checkbox`] for `f`. When the live theme ignores the toggle the
/// row steps back to the dim ink (focus keeps its own colour), with the
/// [`inert`] note at its right end where there is room for both — a canvas
/// card is usually too narrow, and the dim row says it alone.
pub(super) fn checkbox(buf: &mut Buffer, r: Rect, f: Field, on: bool, focused: bool) {
    let label = super::labels::label_of(f);
    form::checkbox(buf, r, label, on, focused);
    let Some(note) = inert(f) else { return };
    if !focused {
        for x in r.x..r.x + r.width {
            if let Some(c) = buf.cell_mut((x, r.y)) {
                c.set_fg(form::dim());
            }
        }
    }
    let w = note.chars().count() as u16;
    if r.height < 1 || r.width < label.chars().count() as u16 + w + 6 {
        return;
    }
    let style = Style::new().fg(form::dim());
    buf.set_line(r.x + r.width - 1 - w, r.y, &Line::styled(note, style), w);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(opacity: f32) -> SettingsPane {
        let mut p = SettingsPane::new(crate::config::CrewConfig::default(), Vec::new());
        p.draft.window_opacity = opacity;
        p
    }

    /// On glass the box says the theme sets the window; on a tube only when
    /// its cap wins; on any other theme it says nothing.
    #[test]
    fn the_box_says_when_the_theme_sets_the_window() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_theme(crew_theme::ThemeId::GlassSky);
        assert_eq!(
            note(&pane(1.0), Field::WindowOpacity).as_deref(),
            Some("glass sets 25")
        );
        assert_eq!(note(&pane(1.0), Field::FontSize), None);
        crew_theme::set_theme(crew_theme::ThemeId::CrtGreen);
        assert_eq!(
            note(&pane(1.0), Field::WindowOpacity).as_deref(),
            Some("tube sets 84")
        );
        assert_eq!(
            note(&pane(0.5), Field::WindowOpacity),
            None,
            "a lower setting wins"
        );
        crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
        assert_eq!(note(&pane(1.0), Field::WindowOpacity), None);
    }

    /// Grain, Paper texture and Drifting background say so on a theme that
    /// leaves them nothing to act on, and stay quiet where they work.
    #[test]
    fn a_canvas_setting_says_when_the_theme_ignores_it() {
        let _g = crate::app::theme_test_guard();
        crew_theme::set_theme(crew_theme::ThemeId::GlassClear);
        assert_eq!(
            inert(Field::PaperGrain).as_deref(),
            Some("no grain on glass")
        );
        assert_eq!(
            inert(Field::PaperTexture).as_deref(),
            Some("no grain on glass")
        );
        let drift = inert(Field::AmbientDrift);
        assert_eq!(drift.as_deref(), Some("no wallpaper on glass"));
        assert_eq!(inert(Field::FontSize), None);
        // The toggle's row steps back to the dim ink.
        let area = Rect::new(0, 0, 30, 1);
        let mut buf = Buffer::empty(area);
        checkbox(&mut buf, area, Field::PaperTexture, true, false);
        let label_fg = buf.cell((4, 0)).map(|c| c.fg);
        assert_eq!(label_fg, Some(form::dim()), "an inert toggle reads dim");
        for id in crew_theme::ALL_THEMES {
            crew_theme::set_theme(id);
            let t = crew_theme::theme();
            let grain = inert(Field::PaperGrain).is_some();
            assert_eq!(grain, t.grain == 0.0, "{}", id.as_str());
        }
    }
}
