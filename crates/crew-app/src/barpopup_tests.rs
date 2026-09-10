use super::item_at;
use crate::layout::Rect;

const CW: f32 = 8.0;
const CH: f32 = 16.0;

/// A 40 × 7 card at (12, 100): five list rows inside the frame.
fn card() -> Rect {
    Rect {
        x: 12.0,
        y: 100.0,
        w: 40.0 * CW,
        h: 7.0 * CH,
    }
}

/// A point on a list row resolves to its item; the frame and the outside
/// resolve to nothing.
#[test]
fn a_point_on_a_row_is_its_item_and_the_frame_is_nothing() {
    let c = card();
    let at = |col: f32, row: f32| item_at(5, 0, c, CW, CH, c.x + col * CW, c.y + row * CH);
    assert_eq!(at(3.5, 1.5), Some(0));
    assert_eq!(at(20.0, 5.5), Some(4));
    assert_eq!(at(3.5, 0.5), None, "the top border");
    assert_eq!(at(3.5, 6.5), None, "the bottom border");
    assert_eq!(at(0.5, 2.5), None, "the left border");
    assert_eq!(at(39.5, 2.5), None, "the right border");
    assert_eq!(at(45.0, 2.5), None, "beside the card");
    assert_eq!(item_at(5, 0, c, CW, CH, 0.0, 0.0), None, "off the card");
}

/// A scrolled list maps its first drawn row through the scroll; a row
/// past the list's end is nothing.
#[test]
fn a_scrolled_list_maps_through_its_offset() {
    let c = Rect {
        h: 12.0 * CH,
        ..card()
    };
    let at = |row: f32, sel: usize| item_at(14, sel, c, CW, CH, c.x + 3.5 * CW, c.y + row * CH);
    assert_eq!(at(1.5, 12), Some(3));
    assert_eq!(at(10.5, 12), Some(12));
    assert_eq!(at(1.5, 0), Some(0));
    let short = Rect {
        h: 5.0 * CH,
        ..card()
    };
    assert_eq!(
        item_at(2, 0, short, CW, CH, short.x + 3.5 * CW, short.y + 3.5 * CH),
        None
    );
}

/// `pick_menu` is the Enter path: a runnable row submits its line and
/// clears the bar; a value-picker command fills and stays.
#[test]
fn pick_menu_is_the_enter_path() {
    let mut bar = crate::inputbar::InputBar {
        text: "/th".into(),
        focused: true,
        ..Default::default()
    };
    let row = |fill: &str, submit: bool| crate::suggest::MenuItem {
        label: fill.into(),
        fill: fill.into(),
        submit,
        ..Default::default()
    };
    bar.menu_sel = 1;
    assert_eq!(
        bar.pick_menu(&[row("/theme ", false), row("/todo", true)]),
        Some("/todo".into())
    );
    assert_eq!(bar.text, "");
    assert_eq!(bar.menu_sel, 0);
    assert_eq!(bar.pick_menu(&[row("/theme ", false)]), None);
    assert_eq!(bar.text, "/theme ");
    assert_eq!(bar.pick_menu(&[]), None, "nothing to pick");
}
