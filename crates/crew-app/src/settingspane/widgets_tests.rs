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
