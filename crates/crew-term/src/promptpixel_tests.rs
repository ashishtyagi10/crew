//! A dead program's DECSET 1016 (SGR-Pixels mouse) is cleared when the shell
//! gets the pane back, or the shell's own clicks would arrive in pixels.
use super::*;
use crate::model::TermCore;
use crate::GridSize;

#[test]
fn pixel_mouse_alone_is_a_leftover_and_the_reset_names_it() {
    let m = InputModes {
        pixel_mouse: true,
        ..InputModes::default()
    };
    assert_eq!(leftover_reset(m, false, false, false), Some(RESET));
    let kitty = InputModes {
        kitty_keys: true,
        ..m
    };
    assert_eq!(
        leftover_reset(kitty, false, false, false),
        Some(RESET_KITTY)
    );
    for r in [RESET, RESET_KITTY] {
        assert!(String::from_utf8_lossy(r).contains(";1016;"));
    }
}

#[test]
fn feeding_the_reset_clears_the_mode() {
    let mut core = TermCore::new(GridSize { cols: 40, rows: 6 });
    core.feed(b"\x1b[?1000h\x1b[?1016h");
    let m = core.input_modes();
    assert!(m.pixel_mouse);
    let reset = leftover_reset(m, false, false, true).expect("a leftover");
    core.feed(reset);
    let after = core.input_modes();
    assert!(!after.pixel_mouse && !after.mouse);
}
