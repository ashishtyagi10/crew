use super::{meta, meta_named};
use crate::keyenc::Mods;
use winit::keyboard::NamedKey;

const ALT: Mods = Mods {
    shift: false,
    alt: true,
    ctrl: false,
};

/// Off a Mac Alt+b is readline's M-b, Alt+. its last-argument key: both
/// arrived as the bare letter, so the shell typed them.
#[test]
fn off_a_mac_alt_letter_is_escape_then_the_letter() {
    assert_eq!(meta("b", "b", ALT, false, false), Some(b"\x1bb".to_vec()));
    assert_eq!(meta(".", ".", ALT, false, false), Some(b"\x1b.".to_vec()));
    let shift = Mods { shift: true, ..ALT };
    assert_eq!(meta("B", "b", shift, false, false), Some(b"\x1bB".to_vec()));
}

/// Ctrl+Alt is AltGr on Windows: `@` on a German keyboard must stay `@`.
#[test]
fn altgr_and_plain_keys_type_as_they_are() {
    let altgr = Mods { ctrl: true, ..ALT };
    assert_eq!(meta("@", "q", altgr, false, false), None);
    assert_eq!(meta("b", "b", Mods::default(), false, false), None);
}

/// On a Mac Option types: with the setting off ∫ stays ∫. With it on, the
/// keycap's letter goes after ESC, not what Option composed.
#[test]
fn on_a_mac_option_is_meta_only_when_asked() {
    assert_eq!(meta("∫", "b", ALT, true, false), None);
    assert_eq!(meta("∫", "b", ALT, true, true), Some(b"\x1bb".to_vec()));
    assert_eq!(meta("≥", ".", ALT, true, true), Some(b"\x1b.".to_vec()));
    let shift = Mods { shift: true, ..ALT };
    assert_eq!(meta("ı", "b", shift, true, true), Some(b"\x1bB".to_vec()));
    // Shift+. is `>`, which the unshifted keycap cannot say.
    assert_eq!(meta("˘", ".", shift, true, true), None);
}

/// Alt+Enter sent a bare CR, so Option+Enter (Claude Code's newline, with
/// Option as Meta on) submitted the prompt. xterm's metaSendsEscape: ESC CR.
#[test]
fn alt_enter_is_escape_then_return() {
    assert_eq!(
        meta_named(NamedKey::Enter, ALT, false, false),
        Some(b"\x1b\r".to_vec())
    );
    assert_eq!(
        meta_named(NamedKey::Enter, ALT, true, true),
        Some(b"\x1b\r".to_vec())
    );
    assert_eq!(
        meta_named(NamedKey::Tab, ALT, false, false),
        Some(b"\x1b\t".to_vec())
    );
    assert_eq!(
        meta_named(NamedKey::Escape, ALT, true, true),
        Some(b"\x1b\x1b".to_vec())
    );
    assert_eq!(
        meta_named(NamedKey::Space, ALT, false, false),
        Some(b"\x1b ".to_vec())
    );
}

/// The Mac setting off, AltGr, Shift+Enter's line feed and the keys
/// `keyenc` already words (Backspace, arrows) are left to the plain path.
#[test]
fn named_meta_keeps_its_gates() {
    assert_eq!(meta_named(NamedKey::Enter, ALT, true, false), None);
    let altgr = Mods { ctrl: true, ..ALT };
    assert_eq!(meta_named(NamedKey::Enter, altgr, false, false), None);
    let shift = Mods { shift: true, ..ALT };
    assert_eq!(meta_named(NamedKey::Enter, shift, false, false), None);
    assert_eq!(
        meta_named(NamedKey::Enter, Mods::default(), false, false),
        None
    );
    assert_eq!(meta_named(NamedKey::Backspace, ALT, false, false), None);
    assert_eq!(meta_named(NamedKey::ArrowLeft, ALT, true, true), None);
}
