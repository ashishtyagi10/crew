//! Off a Mac, no help, welcome or palette string names a key that is not
//! there: the old Ctrl+Shift+T, a Ctrl+Alt chord, or a Mac glyph.
use super::offmac;

const LIES: [&str; 3] = ["Ctrl+Shift+T", "Ctrl+Alt+", "\u{2318}"];

fn clean(what: &str, s: &str) {
    for lie in LIES {
        // `Ctrl+Shift+Tab` is the pane switcher, which is real.
        let s = s.replace("Ctrl+Shift+Tab", "");
        assert!(!s.contains(lie), "{what}: {s}");
    }
}

/// Every row of `/keys`, keys and words, as written off a Mac.
#[test]
fn the_keys_overlay_off_a_mac_names_no_old_chord() {
    let tables = crate::helplayout::sections()
        .into_iter()
        .map(|(_, t)| t)
        .chain([crate::helptable::BINDINGS]);
    for table in tables {
        for (k, d) in table {
            clean(k, &offmac(k));
            clean(d, &offmac(d));
        }
    }
}

#[test]
fn the_welcome_hint_and_the_palette_off_a_mac_name_no_old_chord() {
    for h in crate::welcometext::HINTS_ALT {
        clean(h, h);
        assert!(h.contains("Alt+"), "{h}");
    }
    for cmd in ["/reopen", "/focus", "/settings", "/view", "/smith", "/zoom"] {
        let key = crate::cmdkeys::key_for(cmd).unwrap();
        clean(cmd, &offmac(key));
        assert!(!offmac(key).contains("Cmd+"), "{cmd}: {key}");
    }
}
