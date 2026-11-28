//! The keys off a Mac (`winkeys`) as `keychord` routes them, and a Mac's
//! own, which must not move.
use super::*;
use winit::keyboard::ModifiersState;

const ALT: ModifiersState = ModifiersState::ALT;
const CTRL: ModifiersState = ModifiersState::CONTROL;

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

/// A key press as `cmd_chord` reads it: off a Mac (`mac` false) or on one.
fn cmd(s: &str, mods: ModifiersState, mac: bool) -> Option<String> {
    cmd_string(&ch(s), &ch(s), None, mods, mac)
}

fn special_of(s: &str, mods: ModifiersState, mac: bool) -> Option<WinChord> {
    special(&ch(s), &ch(s), None, mods, mac)
}

/// Off a Mac, Alt is Cmd: split, tab, close, and the remapped three-key ones.
#[test]
fn off_a_mac_alt_is_cmd() {
    for k in ["d", "t", "w", "p", "n", "f", "i", ","] {
        assert_eq!(cmd(k, ALT, false).as_deref(), Some(k), "Alt+{k}");
    }
    assert_eq!(cmd("u", ALT, false).as_deref(), Some("T"), "Alt+U reopens");
    assert_eq!(cmd("x", ALT, false).as_deref(), Some("E"), "Alt+X Explorer");
    assert_eq!(cmd("m", ALT, false).as_deref(), Some("M"), "Alt+M to tab");
    let digit = cmd_string(&ch("1"), &ch("1"), Some(KeyCode::Digit1), ALT, false);
    assert_eq!(digit.as_deref(), Some("1"), "Alt+1 focuses pane 1");
}

/// Plain Ctrl is the terminal's now, whatever the key and wherever the focus:
/// no stand-in turns Ctrl+D, Ctrl+T or Ctrl+W into a chord.
#[test]
fn off_a_mac_plain_ctrl_is_no_chord() {
    for k in ["d", "t", "w", "c", "z", "r", "p", "n", "1", "[", ","] {
        assert_eq!(cmd(k, CTRL, false), None, "Ctrl+{k}");
        let shifted = CTRL | ModifiersState::SHIFT;
        assert_eq!(cmd(k, shifted, false), None, "Ctrl+Shift+{k}");
        assert_eq!(special_of(k, CTRL, false), None, "Ctrl+{k} special");
    }
    assert_eq!(special_of("q", CTRL, false), None, "Ctrl+Q no longer quits");
    for k in ["l", "g", "f", "m"] {
        let shifted = CTRL | ModifiersState::SHIFT;
        assert_eq!(special_of(k, shifted, false), None, "Ctrl+Shift+{k}");
    }
}

/// AltGr arrives as Ctrl+Alt and types its character; the Windows key is the
/// OS's, and Shift alone is a capital.
#[test]
fn off_a_mac_altgr_super_and_shift_are_no_chord() {
    for k in ["q", "d", "t", "@", "e"] {
        assert_eq!(cmd(k, CTRL | ALT, false), None, "AltGr+{k}");
        assert_eq!(special_of(k, CTRL | ALT, false), None, "AltGr+{k}");
    }
    assert_eq!(cmd("t", ModifiersState::SUPER, false), None);
    assert_eq!(cmd("t", ModifiersState::SHIFT, false), None);
    assert_eq!(cmd("t", ModifiersState::empty(), false), None);
    assert_eq!(cmd("t", ALT | ModifiersState::SHIFT, false), None);
}

/// The four walks and quit: Alt+L/B/H/Y/Q off a Mac, Ctrl+Shift+L/G/F/M and
/// Cmd/Ctrl+Q on one, each only on its own platform.
#[test]
fn the_walks_and_quit_have_a_key_per_platform() {
    let cs = CTRL | ModifiersState::SHIFT;
    let off = [
        ("l", WinChord::Theme),
        ("b", WinChord::Gradient),
        ("h", WinChord::Focus),
        ("y", WinChord::MdSource),
        ("q", WinChord::Quit),
    ];
    for (k, want) in off {
        assert_eq!(special_of(k, ALT, false), Some(want), "Alt+{k}");
        assert_eq!(special_of(k, ALT, true), None, "Alt+{k} on a Mac");
    }
    let mac = [
        ("l", WinChord::Theme),
        ("g", WinChord::Gradient),
        ("F", WinChord::Focus),
        ("M", WinChord::MdSource),
    ];
    for (k, want) in mac {
        assert_eq!(special_of(k, cs, true), Some(want), "Ctrl+Shift+{k}");
    }
    assert_eq!(
        special_of("q", ModifiersState::SUPER, true),
        Some(WinChord::Quit)
    );
    assert_eq!(special_of("q", CTRL, true), Some(WinChord::Quit));
    assert_eq!(
        special_of("l", CTRL, true),
        None,
        "Ctrl+L alone is not a walk"
    );
    // Alt+D is a Cmd chord, not one of these.
    assert_eq!(special_of("d", ALT, false), None);
}

/// A Mac's Alt (Option) is never a chord: it types (ß, †) or reaches the
/// terminal as Meta. Only the Cmd key makes a chord there.
#[test]
fn on_a_mac_alt_is_no_chord() {
    for k in ["d", "t", "w", "u", "x", "m", "s", "l"] {
        assert_eq!(cmd(k, ALT, true), None, "Option+{k}");
        assert_eq!(cmd(k, ALT | ModifiersState::SHIFT, true), None);
        assert_eq!(cmd(k, CTRL, true), None, "Ctrl+{k}");
    }
    assert_eq!(cmd("d", ModifiersState::SUPER, true).as_deref(), Some("d"));
    // A held Cmd swallows a key that names no character.
    let named = Key::Named(NamedKey::Enter);
    let s = cmd_string(&named, &named, None, ModifiersState::SUPER, true);
    assert_eq!(s.as_deref(), Some(""));
}

/// Off a Mac Ctrl+D, Ctrl+T and Ctrl+W make no chord, so they reach the
/// terminal, whose encoding turns them into EOF, 0x14 and 0x17.
#[test]
fn off_a_mac_ctrl_d_t_w_reach_the_terminal_as_control_bytes() {
    for (k, byte) in [('d', 0x04), ('t', 0x14), ('w', 0x17)] {
        assert_eq!(cmd(&k.to_string(), CTRL, false), None);
        assert_eq!(crate::session::ctrl_byte(k), Some(byte));
    }
}
