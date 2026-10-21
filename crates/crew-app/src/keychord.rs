//! Two key *predicates* pulled out of [`crate::keys`]'s dispatch: the arrow
//! that names a grid direction, and the chord that toggles a chat pane's
//! compact view. Both are pure functions over a `winit` key so the mapping is
//! testable without constructing a `KeyEvent`.
use winit::keyboard::{Key, ModifiersState, NamedKey};

/// The arrow direction a key names, or `None` for every other key. Kept as a
/// free function (like [`is_compact_chord`]) so the mapping is testable
/// without building a winit `KeyEvent`.
pub(crate) fn arrow_dir(key: &Key) -> Option<crate::panedir::Dir> {
    use crate::panedir::Dir;
    match key {
        Key::Named(NamedKey::ArrowLeft) => Some(Dir::Left),
        Key::Named(NamedKey::ArrowRight) => Some(Dir::Right),
        Key::Named(NamedKey::ArrowUp) => Some(Dir::Up),
        Key::Named(NamedKey::ArrowDown) => Some(Dir::Down),
        _ => None,
    }
}

/// Ctrl+O — the chord that toggles a chat pane's compact transcript view.
/// Extracted as a pure predicate (mirrors `swarmpane::esc_closes`) so the
/// match is testable without constructing a winit `KeyEvent`. Modeled on the
/// Ctrl+Shift+M intercept above: same reach (fires before the input-bar
/// early-return), but with no Shift requirement.
pub(crate) fn is_compact_chord(key: &Key, mods: winit::keyboard::ModifiersState) -> bool {
    // Not with Shift: off a Mac, Ctrl+Shift+O is Cmd+O (`stand_in`).
    mods.control_key()
        && !mods.shift_key()
        && matches!(key, Key::Character(s) if s.eq_ignore_ascii_case("o"))
}

/// The Cmd chord a key press makes, if any: the character a held Cmd (Super)
/// gives — empty for a key that names none, which a held Cmd still swallows
/// — or, off a Mac, the [`stand_in`] for it. `typed`: a terminal or a line
/// being typed has the focus (`ctrlchord`).
pub(crate) fn cmd_chord(
    event: &winit::event::KeyEvent,
    mods: ModifiersState,
    typed: bool,
) -> Option<String> {
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    if !event.state.is_pressed() {
        return None;
    }
    if mods.super_key() {
        return Some(match &event.logical_key {
            Key::Character(s) => s.to_string(),
            _ => String::new(),
        });
    }
    stand_in(
        &event.key_without_modifiers(),
        mods,
        cfg!(target_os = "macos"),
        typed,
    )
}

/// What a terminal, or a line being typed, keeps for itself from plain Ctrl:
/// interrupt, end of input, suspend, history search, clear, line start and
/// end, kill — and the keys that ARE Enter, newline, Tab, Backspace and
/// Escape to a terminal (M, J, I, H, `[`).
pub(crate) const TYPED: &str = "cdzrlaekumjih[";

/// Off a Mac the Cmd key is the Windows key, and the OS keeps most of its
/// chords (Win+I opens Settings, Win+T the taskbar). So plain Ctrl+<key> is
/// Cmd+<key>, as a browser or an editor has it (the user, 2026-10-21: "cmd+t
/// works on mac but ctrl+t doesn't work on windows"), except a key in
/// [`TYPED`] while a terminal or a typed line has the focus. Ctrl+Shift+<key>
/// reaches Cmd+<key> everywhere, as Windows Terminal has it; and where plain
/// Ctrl already makes a chord a Mac shifts, the shifted one (Ctrl+Shift+T
/// reopens a pane, as a browser reopens a tab). `unshifted` is the key as if
/// no modifier were held, so Ctrl+Shift+1 is Cmd+1, not `!`.
pub(crate) fn stand_in(
    unshifted: &Key,
    mods: ModifiersState,
    mac: bool,
    typed: bool,
) -> Option<String> {
    let ctrl = mods.control_key() && !mods.alt_key() && !mods.super_key();
    let Key::Character(s) = unshifted else {
        return None;
    };
    if mac || !ctrl {
        return None;
    }
    let k = s.to_lowercase();
    match mods.shift_key() {
        true if stands_in(&k) => Some(shifted(&k).to_string()),
        true => None,
        false if plain(&k, typed) => Some(k),
        false => None,
    }
}

/// The chord a Mac makes with Shift on `k`, where plain Ctrl+`k` is taken
/// by the unshifted one; `k` itself for the rest.
fn shifted(k: &str) -> &str {
    match k {
        "t" => "T",
        "[" => "{",
        "]" => "}",
        k => k,
    }
}

/// Whether plain Ctrl+`key` is Cmd+`key`: one key crew's chords use, and not
/// one a terminal or a typed line keeps ([`TYPED`]) while one has the focus.
pub(crate) fn plain(key: &str, typed: bool) -> bool {
    let mut chars = key.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return false;
    };
    let c = c.to_ascii_lowercase();
    (c.is_ascii_alphanumeric() || "=-,./[]".contains(c)) && !(typed && TYPED.contains(c))
}

/// Whether Cmd+`key` has a Ctrl+Shift stand-in: one unshifted key, and not
/// one of the Ctrl+Shift chords crew already gives a meaning on every
/// platform (L themes, G gradient, F focus mode, M markdown source).
pub(crate) fn stands_in(key: &str) -> bool {
    let mut chars = key.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return false;
    };
    let c = c.to_ascii_lowercase();
    (c.is_ascii_alphanumeric() || "=-,./[]".contains(c)) && !"lgfm".contains(c)
}

#[cfg(test)]
#[path = "keychord_tests.rs"]
mod tests;
