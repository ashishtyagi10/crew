//! Which Ctrl keys a pane keeps off a Mac, and that a Mac keeps none.
use super::*;

fn ch(s: &str) -> Key {
    Key::Character(s.into())
}

/// Ctrl+C, V, S and F are the four; every other Ctrl letter is the program's.
#[test]
fn off_a_mac_ctrl_c_v_s_f_are_the_panes_and_the_rest_are_not() {
    let want = [
        ("c", CtrlKey::Copy),
        ("v", CtrlKey::Paste),
        ("s", CtrlKey::Save),
        ("f", CtrlKey::Find),
    ];
    for (k, w) in want {
        assert_eq!(ctrl_key(&ch(k), ModifiersState::CONTROL, false), Some(w));
    }
    for k in ["a", "d", "z", "r", "l", "o", "w", "u", "e", "k", "x", "y"] {
        assert_eq!(
            ctrl_key(&ch(k), ModifiersState::CONTROL, false),
            None,
            "{k}"
        );
    }
}

/// Mac: never. Ctrl+C there is ^C with or without a selection, as before.
#[test]
fn on_a_mac_no_ctrl_key_is_the_panes() {
    for k in ["c", "v", "s", "f"] {
        assert_eq!(ctrl_key(&ch(k), ModifiersState::CONTROL, true), None, "{k}");
    }
}

/// Shift, Alt (AltGr is Ctrl+Alt, which types) or Super alongside make no Ctrl key.
#[test]
fn another_modifier_makes_it_something_else() {
    let c = ModifiersState::CONTROL;
    for m in [
        c | ModifiersState::SHIFT,
        c | ModifiersState::ALT,
        c | ModifiersState::SUPER,
    ] {
        assert_eq!(ctrl_key(&ch("v"), m, false), None, "{m:?}");
    }
    assert_eq!(ctrl_key(&ch("v"), ModifiersState::empty(), false), None);
    assert_eq!(ctrl_key(&ch("v"), ModifiersState::ALT, false), None);
}

/// The mail window asks only for the paste.
#[test]
fn the_mail_window_takes_ctrl_v_as_its_paste_chord() {
    let c = ModifiersState::CONTROL;
    assert_eq!(paste_chord(&ch("v"), c, false).as_deref(), Some("v"));
    assert_eq!(
        paste_chord(&ch("w"), c, false),
        None,
        "Ctrl+W closes nothing"
    );
    assert_eq!(paste_chord(&ch("v"), c, true), None);
}

/// What a Mac does with Ctrl+C is the byte, selection or no selection: the
/// new route never claims it, so the terminal encoding below is what it gets.
#[test]
fn on_a_mac_ctrl_c_still_encodes_as_the_interrupt_byte() {
    assert_eq!(
        crate::session::typed_bytes(&ch("c"), None, true).as_deref(),
        Some(&[0x03][..])
    );
    assert_eq!(ctrl_key(&ch("c"), ModifiersState::CONTROL, true), None);
}

/// Off a Mac, Ctrl+F in an editing pane asks for the find (Alt+F's), in a
/// reading pane or with the bar focused it is nobody's; Ctrl+C with nothing
/// selected is nobody's too, so a terminal gets its ^C.
#[test]
fn ctrl_f_finds_from_an_editing_pane_only() {
    use crate::app::CrewApp;
    use crate::pane::PaneContent;
    let dir = std::env::temp_dir().join("ctrlpane-tests");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("f.md"), "# Title\n\nbody\n").unwrap();
    let mut app = CrewApp {
        cwd: dir,
        ..Default::default()
    };
    app.open_view("f.md");
    for _ in 0..500 {
        if let PaneContent::View(v) = &mut app.panes[0].content {
            v.poll();
            if !v.loading() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    app.focused = 0;
    app.input.focused = false;
    assert!(!app.run_ctrl_key(CtrlKey::Find), "a reader has no Ctrl+F");
    let (cols, rows) = (app.panes[0].grid.cols, app.panes[0].grid.rows);
    if let PaneContent::View(v) = &mut app.panes[0].content {
        v.edit_here(cols, rows);
    }
    assert!(app.run_ctrl_key(CtrlKey::Find));
    assert!(app.input.text.starts_with("/find"), "{:?}", app.input.text);
    app.input.focused = true;
    assert!(!app.run_ctrl_key(CtrlKey::Find), "the bar has the keys");
    assert!(!app.run_ctrl_key(CtrlKey::Copy), "nothing selected");
}
