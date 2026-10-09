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
        return None;
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
}
