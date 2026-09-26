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
