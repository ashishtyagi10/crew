use super::checkbox;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn drawn(on: bool) -> Buffer {
    let area = Rect::new(0, 0, 24, 1);
    let mut buf = Buffer::empty(area);
    checkbox(&mut buf, area, "Paper texture", on, false);
    buf
}

fn text(buf: &Buffer) -> String {
    (0..24).map(|x| buf[(x, 0)].symbol().to_string()).collect()
}

/// A checkbox is a drawn box — crew's own `□`, filled `■` in the accent when
/// on — not `[x]` typed into the form, which read as text rather than a
/// control (the same fix the pane buttons had).
#[test]
fn a_checkbox_is_a_drawn_box_filled_when_on() {
    let _g = crate::app::theme_test_guard();
    let (on, off) = (drawn(true), drawn(false));
    assert_eq!(text(&on).trim_end(), "  \u{25a0} Paper texture");
    assert_eq!(text(&off).trim_end(), "  \u{25a1} Paper texture");
    let accent = crate::palette::focus_color();
    assert_eq!(on[(2, 0)].fg, accent, "the filled box wears the accent");
    assert_ne!(off[(2, 0)].fg, accent, "an empty box stays quiet");
}

/// Save and Cancel are buttons, not `[ Save ⌘S ]` text: each a filled
/// capsule — Save on the accent, the one to press; Cancel on a quiet tint —
/// its label clearing the text floor on its fill. Same widths as the old
/// brackets, so the click targets have not moved.
#[test]
fn save_and_cancel_are_filled_buttons() {
    let _g = crate::app::theme_test_guard();
    use super::{button, CANCEL, SAVE};
    assert_eq!(SAVE.chars().count(), "[ Save \u{2318}S ]".chars().count());
    assert_eq!(CANCEL.chars().count(), "[ Cancel esc ]".chars().count());
    assert!(!SAVE.contains('[') && !CANCEL.contains('['));
    assert!(
        !SAVE.contains(' ') && !CANCEL.contains(' '),
        "no bare spaces: they square the ends"
    );
    let page = crew_theme::theme().page_bg;
    let rgb = |c: Option<ratatui::style::Color>| match c {
        Some(ratatui::style::Color::Rgb(r, g, b)) => (r, g, b),
        other => panic!("{other:?}"),
    };
    for (text, primary) in [(SAVE, true), (CANCEL, false)] {
        let s = button(text, false, primary);
        let (fg, bg) = (rgb(s.style.fg), rgb(s.style.bg));
        assert_ne!(bg, page, "{text:?} has a fill");
        let floor = crew_theme::contrast::text_floor();
        assert!(
            crew_theme::contrast_ratio(fg, bg) >= floor,
            "{text:?} reads"
        );
    }
    let (save, cancel) = (button(SAVE, false, true), button(CANCEL, false, false));
    assert_ne!(save.style.bg, cancel.style.bg, "Save is the primary one");
}

/// A label too wide for its card ends in `…`, never mid-word.
#[test]
fn a_clipped_checkbox_label_says_so() {
    let _g = crate::app::theme_test_guard();
    let area = Rect::new(0, 0, 12, 1);
    let mut buf = Buffer::empty(area);
    checkbox(&mut buf, area, "Drifting background", true, false);
    let row: String = (0..12).map(|x| buf[(x, 0)].symbol().to_string()).collect();
    assert_eq!(row, "  \u{25a0} Driftin\u{2026}");
}
