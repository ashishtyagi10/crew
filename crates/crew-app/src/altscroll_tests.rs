use super::*;

fn modes() -> InputModes {
    InputModes::default()
}

#[test]
fn plain_shell_keeps_local_scrollback() {
    // No alt-screen, no mouse → None, so Crew scrolls its own history.
    assert_eq!(wheel_bytes(&modes(), 3, (0, 0)), None);
    assert_eq!(wheel_bytes(&modes(), -3, (0, 0)), None);
}

#[test]
fn zero_lines_is_noop() {
    let m = InputModes {
        alt_screen: true,
        alternate_scroll: true,
        ..modes()
    };
    assert_eq!(wheel_bytes(&m, 0, (0, 0)), None);
}

#[test]
fn alt_screen_sends_arrow_keys() {
    let m = InputModes {
        alt_screen: true,
        alternate_scroll: true,
        ..modes()
    };
    // Two ticks up → two cursor-up keys (normal cursor mode).
    assert_eq!(wheel_bytes(&m, 2, (5, 5)), Some(b"\x1b[A\x1b[A".to_vec()));
    // Down → cursor-down.
    assert_eq!(wheel_bytes(&m, -1, (5, 5)), Some(b"\x1b[B".to_vec()));
}

#[test]
fn app_cursor_uses_ss3_arrows() {
    let m = InputModes {
        alt_screen: true,
        alternate_scroll: true,
        app_cursor: true,
        ..modes()
    };
    assert_eq!(wheel_bytes(&m, 1, (0, 0)), Some(b"\x1bOA".to_vec()));
}

#[test]
fn alt_screen_without_alternate_scroll_falls_back() {
    let m = InputModes {
        alt_screen: true,
        alternate_scroll: false,
        ..modes()
    };
    assert_eq!(wheel_bytes(&m, 1, (0, 0)), None);
}

#[test]
fn mouse_mode_emits_sgr_wheel_at_one_based_cell() {
    let m = InputModes {
        mouse: true,
        sgr_mouse: true,
        ..modes()
    };
    // Hovered cell (3,5) → 1-based (4,6); wheel up = button 64.
    assert_eq!(wheel_bytes(&m, 1, (3, 5)), Some(b"\x1b[<64;4;6M".to_vec()));
    // Wheel down = button 65.
    assert_eq!(wheel_bytes(&m, -1, (3, 5)), Some(b"\x1b[<65;4;6M".to_vec()));
}

#[test]
fn mouse_mode_legacy_encoding_offsets_by_32() {
    let m = InputModes {
        mouse: true,
        sgr_mouse: false,
        ..modes()
    };
    // button 64 -> 96, x=1 -> 33, y=1 -> 33.
    assert_eq!(
        wheel_bytes(&m, 1, (0, 0)),
        Some(vec![0x1b, b'[', b'M', 96, 33, 33])
    );
}

#[test]
fn wheel_ticks_are_capped() {
    let m = InputModes {
        alt_screen: true,
        alternate_scroll: true,
        ..modes()
    };
    let bytes = wheel_bytes(&m, 1000, (0, 0)).unwrap();
    assert_eq!(bytes.len(), MAX_TICKS as usize * 3); // 3 bytes per arrow
}

#[test]
fn page_bytes_only_in_alt_screen() {
    assert_eq!(page_bytes(&modes(), true), None);
    let alt = InputModes {
        alt_screen: true,
        ..modes()
    };
    assert_eq!(page_bytes(&alt, true), Some(b"\x1b[5~".to_vec()));
    assert_eq!(page_bytes(&alt, false), Some(b"\x1b[6~".to_vec()));
}
