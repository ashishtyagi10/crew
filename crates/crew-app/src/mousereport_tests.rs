use super::*;
use crew_term::{GridSize, HeadlessTerm, TermModel};

fn modes_after(bytes: &[u8]) -> InputModes {
    let mut t = HeadlessTerm::new(GridSize { cols: 80, rows: 24 });
    t.feed(bytes);
    t.input_modes()
}

#[test]
fn sgr_press_and_release_are_one_based_and_keep_the_button() {
    assert_eq!(report(0, (4, 2), true, true), b"\x1b[<0;5;3M");
    assert_eq!(report(0, (4, 2), true, false), b"\x1b[<0;5;3m");
    assert_eq!(report(2, (0, 0), true, true), b"\x1b[<2;1;1M");
}

#[test]
fn modifiers_ride_on_the_button_but_shift_never_does() {
    assert_eq!(mod_bits(false, false), 0);
    assert_eq!(mod_bits(true, false), 8);
    assert_eq!(mod_bits(false, true), 16);
    let ctrl_left = mod_bits(false, true) | button_code(MouseButton::Left).unwrap();
    assert_eq!(report(ctrl_left, (9, 9), true, true), b"\x1b[<16;10;10M");
}

#[test]
fn legacy_x10_offsets_by_32_and_releases_as_button_3() {
    assert_eq!(
        report(0, (4, 2), false, true),
        [0x1b, b'[', b'M', 32, 37, 35]
    );
    assert_eq!(
        report(0, (4, 2), false, false),
        [0x1b, b'[', b'M', 35, 37, 35]
    );
    // A Ctrl-held release keeps its modifier bits: 16 + 3.
    assert_eq!(report(16, (0, 0), false, false)[3], 32 + 19);
    // Past column 223 a field cannot fit in one byte; it clamps there.
    assert_eq!(report(0, (300, 0), false, true)[4], 255);
}

#[test]
fn buttons_map_to_xterm_numbers() {
    assert_eq!(button_code(MouseButton::Left), Some(0));
    assert_eq!(button_code(MouseButton::Middle), Some(1));
    assert_eq!(button_code(MouseButton::Right), Some(2));
    assert_eq!(button_code(MouseButton::Back), None);
}

#[test]
fn a_program_that_asked_for_the_mouse_gets_the_click_unless_shift() {
    // htop / vim mouse=a / tmux: 1000 (clicks) + 1006 (SGR).
    let m = modes_after(b"\x1b[?1000h\x1b[?1006h");
    assert!(m.sgr_mouse);
    assert!(reports_click(&m, false));
    assert!(!reports_click(&m, true), "Shift keeps crew's selection");
    // Drag (1002) and motion (1003) modes report presses too.
    assert!(reports_click(&modes_after(b"\x1b[?1002h"), false));
    assert!(reports_click(&modes_after(b"\x1b[?1003h"), false));
}

#[test]
fn a_plain_shell_keeps_crews_own_click() {
    assert!(!reports_click(&modes_after(b"$ ls\r\n"), false));
    // A full-screen program that never asked for the mouse (less) too.
    assert!(!reports_click(&modes_after(b"\x1b[?1049h"), false));
    // And one that turned it back off on the way out.
    assert!(!reports_click(
        &modes_after(b"\x1b[?1000h\x1b[?1000l"),
        false
    ));
}

#[test]
fn moves_carry_the_motion_bit_only_where_asked() {
    let click = modes_after(b"\x1b[?1000h");
    let drag = modes_after(b"\x1b[?1002h");
    let any = modes_after(b"\x1b[?1003h");
    assert!(drag.mouse_drag && !drag.mouse_motion);
    assert!(any.mouse_motion);
    // Left held: 0 + 32 under drag or any-motion; nothing under clicks only.
    assert_eq!(motion_code(&click, Some(0), 0), None);
    assert_eq!(motion_code(&drag, Some(0), 0), Some(32));
    assert_eq!(motion_code(&any, Some(2), 0), Some(34));
    // No button: 3 + 32, and only under any-motion; modifiers ride along.
    assert_eq!(motion_code(&drag, None, 0), None);
    assert_eq!(motion_code(&any, None, 0), Some(35));
    assert_eq!(motion_code(&any, None, 16), Some(51));
    // The held press's own Ctrl bit is replaced by the modifiers held now.
    assert_eq!(motion_code(&drag, Some(16), 0), Some(32));
}
