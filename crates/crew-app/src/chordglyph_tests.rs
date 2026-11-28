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

/// On a Mac a chord is drawn as glyphs; off one, as the key it stands in
/// for there: a chord a Mac shifts has a key of its own (`offmac`).
#[test]
fn shown_follows_the_platform() {
    let want = if cfg!(target_os = "macos") {
        "\u{21e7}\u{2318}T"
    } else {
        "Alt+U"
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
        // Alt is crew's key off a Mac (`winkeys`).
        false => "no shell here \u{2014} press Alt+T to open one",
    };
    assert_eq!(prose(s), want);
    assert_eq!(prose("no pane is waiting"), "no pane is waiting");
    assert_eq!(
        mac("press Cmd+T to open one"),
        "press \u{2318}T to open one"
    );
}

/// Off a Mac the table's Cmd chords are written as they are pressed there:
/// Alt for Cmd, a key of their own for the three-key ones.
#[test]
fn off_a_mac_cmd_chords_are_written_as_they_are_pressed_there() {
    assert_eq!(offmac("Cmd+I / Cmd+T"), "Alt+I / Alt+T");
    assert_eq!(offmac("Cmd+1 \u{2026} 9"), "Alt+1 \u{2026} 9");
    assert_eq!(
        offmac("Cmd+= / Cmd+- / Cmd+0 / Cmd+wheel"),
        "Alt+= / Alt+- / Alt+0 / Ctrl+wheel"
    );
    // A click and the wheel are still Ctrl's (`events::open_modifier`).
    assert_eq!(offmac("Cmd+Click"), "Ctrl+Click");
    assert_eq!(offmac("press Cmd+T, or /new"), "press Alt+T, or /new");
    assert_eq!(offmac("Cmd+C / Cmd+V"), "Alt+C / Alt+V");
    assert_eq!(offmac("Cmd+G / Cmd+Z"), "Alt+G / Alt+Z");
    assert_eq!(offmac("Cmd+W / Cmd+M"), "Alt+W / F11");
    assert_eq!(offmac("Cmd+Enter"), "Alt+Enter");
    // What a Mac shifts, a key of its own; the grid's arrows, Alt.
    assert_eq!(offmac("Cmd+Shift+T"), "Alt+U");
    assert_eq!(offmac("Cmd+Shift+E"), "Alt+X");
    assert_eq!(offmac("Cmd+Shift+M"), "Alt+M");
    assert_eq!(offmac("Cmd+{ / Cmd+}"), "Ctrl+PgUp / Ctrl+PgDn");
    assert_eq!(offmac("Cmd+Opt+1 \u{2026} 9"), "Ctrl+1 \u{2026} 9");
    assert_eq!(offmac("Cmd+Opt+\u{2191}\u{2193}"), "Alt+PgUp / Alt+PgDn");
    assert_eq!(offmac("Cmd+Opt+\u{2190}\u{2192}"), "Alt+Home / Alt+End");
    assert_eq!(
        offmac("Cmd+\u{2190}\u{2191}\u{2192}\u{2193}"),
        "Alt+\u{2190}\u{2191}\u{2192}\u{2193}"
    );
    assert_eq!(
        offmac("Cmd+Shift+\u{2190}\u{2191}\u{2192}\u{2193}"),
        "Alt+Shift+\u{2190}\u{2191}\u{2192}\u{2193}"
    );
    // The four Mac Ctrl+Shift chords.
    assert_eq!(offmac("Ctrl+Shift+L"), "Alt+L");
    assert_eq!(offmac("Ctrl+Shift+G / Ctrl+Shift+F"), "Alt+B / Alt+H");
    assert_eq!(offmac("Ctrl+Shift+M"), "Alt+Y");
    assert_eq!(
        offmac("Ctrl+Tab / Ctrl+Shift+Tab"),
        "Ctrl+Tab / Ctrl+Shift+Tab"
    );
    assert_eq!(offmac("Cmd+S / Alt+S"), "Alt+S", "one key, said once");
}

/// A longer chord is not eaten by a shorter one it ends with.
#[test]
fn a_remap_matches_whole_chords_only() {
    assert_eq!(offmac("Cmd+Moon"), "Alt+Moon");
    assert_eq!(offmac("Cmd+M, then Cmd+Shift+M"), "F11, then Alt+M");
}

/// The Mac's words are the portable table's, glyphs and all, whatever the
/// Windows keys are: this is the Mac side of the label, untouched.
#[test]
fn the_mac_side_is_unchanged_by_the_windows_labels() {
    assert_eq!(mac("Cmd+Shift+T"), "\u{21e7}\u{2318}T");
    assert_eq!(mac("Cmd+M"), "\u{2318}M");
    assert_eq!(mac("Ctrl+Shift+L"), "\u{2303}\u{21e7}L");
    assert_eq!(mac("Cmd+{ / Cmd+}"), "\u{2318}{ / \u{2318}}");
    if cfg!(target_os = "macos") {
        assert_eq!(shown("Cmd+Shift+T"), "\u{21e7}\u{2318}T");
        assert_eq!(
            shown("Cmd+Opt+\u{2190}\u{2192}"),
            "\u{2325}\u{2318}\u{2190}\u{2192}"
        );
        let s = "press Cmd+T, or Cmd+Shift+[ ]";
        assert_eq!(prose_off_mac(s), s, "a Mac's own sentence is left as it is");
    }
}

/// Redo in an editor is Ctrl+Y off a Mac, and every chord there is Ctrl.
#[test]
fn the_editors_write_redo_as_ctrl_y() {
    if !cfg!(target_os = "macos") {
        assert_eq!(shown_ctrl("Cmd+Z / Cmd+Shift+Z"), "Ctrl+Z / Ctrl+Y");
    }
}

/// Every table row, written off a Mac, is at most two keys — save the grid's
/// Alt+Shift+arrow swap — and names no key a Mac has and Windows hasn't.
#[test]
fn no_row_is_written_with_three_keys_off_a_mac() {
    use crate::helptable::BINDINGS;
    for (keys, _) in BINDINGS.iter().chain(crate::helptable::CHAT_BINDINGS) {
        let w = offmac(keys);
        for bad in ["Ctrl+Alt+", "\u{2318}", "Cmd+", "Opt+"] {
            assert!(!w.contains(bad), "{keys} \u{2192} {w}");
        }
        for tok in w.split(' ') {
            let mods = tok.matches("Ctrl+").count()
                + tok.matches("Alt+").count()
                + tok.matches("Shift+").count();
            let exempt = tok.starts_with("Alt+Shift+\u{2190}") || tok == "Ctrl+Shift+Tab";
            assert!(mods < 2 || exempt, "{keys} \u{2192} {w}: {tok}");
        }
    }
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
