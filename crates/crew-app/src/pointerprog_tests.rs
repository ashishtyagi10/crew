use super::css_icon;
use crate::pointer::{icon, Over};
use winit::window::CursorIcon;

#[test]
fn css_names_map_to_winit_icons() {
    for (name, want) in [
        ("pointer", CursorIcon::Pointer),
        ("text", CursorIcon::Text),
        ("crosshair", CursorIcon::Crosshair),
        ("grab", CursorIcon::Grab),
        ("grabbing", CursorIcon::Grabbing),
        ("not-allowed", CursorIcon::NotAllowed),
        ("ew-resize", CursorIcon::EwResize),
        ("ns-resize", CursorIcon::NsResize),
        ("col-resize", CursorIcon::ColResize),
        ("row-resize", CursorIcon::RowResize),
        ("wait", CursorIcon::Wait),
        ("progress", CursorIcon::Progress),
        ("help", CursorIcon::Help),
        ("move", CursorIcon::Move),
        ("zoom-in", CursorIcon::ZoomIn),
    ] {
        assert_eq!(css_icon(name), Some(want), "{name}");
    }
}

#[test]
fn an_unknown_name_keeps_crews_default_and_a_fallback_list_takes_the_first_known() {
    assert_eq!(css_icon("banana"), None);
    assert_eq!(css_icon(""), None);
    assert_eq!(css_icon("banana, hand ,pointer"), Some(CursorIcon::Pointer));
}

#[test]
fn the_programs_shape_is_what_the_pointer_wears() {
    assert_eq!(
        icon(Over::Program(CursorIcon::Crosshair)),
        CursorIcon::Crosshair
    );
}
