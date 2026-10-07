//! A Mac's chord glyphs, in a Mac's order.
use super::*;

#[test]
fn modifiers_become_glyphs_in_the_order_a_mac_writes_them() {
    assert_eq!(mac("Cmd+T"), "\u{2318}T");
    assert_eq!(mac("Cmd+Shift+T"), "\u{21e7}\u{2318}T");
    assert_eq!(
        mac("Shift+Cmd+T"),
        "\u{21e7}\u{2318}T",
        "order is the Mac's, not the table's"
    );
    assert_eq!(mac("Ctrl+Shift+L"), "\u{2303}\u{21e7}L");
    assert_eq!(mac("Ctrl+Alt+Cmd+X"), "\u{2303}\u{2325}\u{2318}X");
}

#[test]
fn several_chords_and_the_words_between_them() {
    assert_eq!(mac("Cmd+] / Cmd+["), "\u{2318}] / \u{2318}[");
    assert_eq!(
        mac("Ctrl+Tab / Ctrl+Shift+Tab"),
        "\u{2303}Tab / \u{2303}\u{21e7}Tab"
    );
    assert_eq!(mac("Cmd+1 \u{2026} 9"), "\u{2318}1 \u{2026} 9");
    assert_eq!(
        mac("Cmd+= / Cmd+- / Cmd+0 / Cmd+wheel"),
        "\u{2318}= / \u{2318}- / \u{2318}0 / \u{2318}wheel"
    );
}

#[test]
fn what_is_not_a_chord_is_left_alone() {
    for s in [
        "Double-click / Triple-click",
        "Tab / \u{2192} (in input)",
        "! \u{b7} * \u{b7} ?",
        "a+b",
        "Cmd+",
    ] {
        assert_eq!(mac(s), s);
    }
}

/// Off a Mac the tables are drawn as written; on one, as glyphs.
#[test]
fn shown_follows_the_platform() {
    let want = if cfg!(target_os = "macos") {
        "\u{21e7}\u{2318}T"
    } else {
        "Cmd+Shift+T"
    };
    assert_eq!(shown("Cmd+Shift+T"), want);
    assert_eq!(shown("Esc"), "Esc");
}

/// A status or toast sentence keeps its words and rewrites only its chords.
#[test]
fn prose_rewrites_only_the_chords() {
    let s = "no shell here \u{2014} press Cmd+T to open one";
    let want = match cfg!(target_os = "macos") {
        true => "no shell here \u{2014} press \u{2318}T to open one",
        false => "no shell here \u{2014} press Ctrl+Shift+T to open one",
    };
    assert_eq!(prose(s), want);
    assert_eq!(prose("no pane is waiting"), "no pane is waiting");
    assert_eq!(
        mac("press Cmd+T to open one"),
        "press \u{2318}T to open one"
    );
}

/// Off a Mac the table's Cmd chords are written as they are pressed there.
#[test]
fn off_a_mac_cmd_chords_are_written_as_their_ctrl_shift_stand_ins() {
    assert_eq!(offmac("Cmd+I / Cmd+T"), "Ctrl+Shift+I / Ctrl+Shift+T");
    assert_eq!(offmac("Cmd+1 \u{2026} 9"), "Ctrl+Shift+1 \u{2026} 9");
    assert_eq!(
        offmac("Cmd+= / Cmd+- / Cmd+0 / Cmd+wheel"),
        "Ctrl+Shift+= / Ctrl+Shift+- / Ctrl+Shift+0 / Ctrl+wheel"
    );
    assert_eq!(offmac("Cmd+Click"), "Ctrl+Click");
    assert_eq!(
        offmac("press Cmd+T, or /new"),
        "press Ctrl+Shift+T, or /new"
    );
    // No stand-in: Ctrl+Shift+G already steps the gradient, and a shifted
    // chord has no Shift left to give.
    assert_eq!(offmac("Cmd+G / Cmd+Z"), "Cmd+G / Ctrl+Shift+Z");
    assert_eq!(offmac("Cmd+Shift+T"), "Cmd+Shift+T");
    assert_eq!(offmac("Cmd+{ / Cmd+}"), "Cmd+{ / Cmd+}");
    assert_eq!(offmac("Ctrl+Shift+L"), "Ctrl+Shift+L", "not a Cmd chord");
}

/// The document window takes plain Ctrl off a Mac, so its own notes say so
/// — never the main window's Ctrl+Shift stand-in.
#[test]
fn the_document_windows_notes_name_its_own_chords() {
    let note = crate::docwin::reread::guard(true, false).unwrap_err();
    if cfg!(target_os = "macos") {
        assert!(note.contains("\u{2318}S"), "{note}");
    } else {
        assert!(note.contains("Ctrl+S") && !note.contains("Shift"), "{note}");
    }
    assert_eq!(prose(note), note, "and the status line leaves it be");
}
