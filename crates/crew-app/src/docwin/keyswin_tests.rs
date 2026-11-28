//! The editor's Ctrl keys off a Mac, where Ctrl is the editing key.
use super::*;

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

const CTRL: ModifiersState = ModifiersState::CONTROL;

/// Ctrl+Z/Y/S/A/C/X/V/B/I/R all reach the editor, on either platform's rule.
#[test]
fn ctrl_editing_keys_reach_the_editor_off_a_mac() {
    let want = [
        ("z", Edit::Undo),
        ("y", Edit::Redo),
        ("s", Edit::Save),
        ("a", Edit::SelectAll),
        ("c", Edit::Copy),
        ("x", Edit::Cut),
        ("v", Edit::Paste),
        ("b", Edit::Wrap("**")),
        ("i", Edit::Wrap("*")),
    ];
    for (k, e) in want {
        assert_eq!(edit_on(&ch(k), true, CTRL, 20, false), Some(e), "Ctrl+{k}");
    }
    // Ctrl+Shift+Z is redo too, as on every platform.
    let shift = CTRL | ModifiersState::SHIFT;
    assert_eq!(edit_on(&ch("Z"), true, shift, 20, false), Some(Edit::Redo));
}

/// Ctrl+Y is redo off a Mac only; the Mac keeps Cmd+Shift+Z and gives Y nothing.
#[test]
fn ctrl_y_is_redo_only_off_a_mac() {
    assert_eq!(edit_on(&ch("y"), true, CTRL, 20, true), None);
    assert_eq!(
        edit_on(&ch("y"), true, ModifiersState::SUPER, 20, true),
        None
    );
    assert_eq!(
        edit_on(&ch("y"), true, ModifiersState::SUPER, 20, false),
        None
    );
    let z = edit_on(
        &ch("Z"),
        true,
        ModifiersState::SUPER | ModifiersState::SHIFT,
        20,
        true,
    );
    assert_eq!(z, Some(Edit::Redo));
}
