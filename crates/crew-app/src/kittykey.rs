//! A winit key event, adapted for the kitty encoders (`keykitty`'s flag 1,
//! `kittyreport`'s 2/4/8/16), and the memory that lets a release be
//! forwarded only for a press the program saw.
use crate::keyenc::Mods;
use crate::keykitty::{armed, encode};
use winit::event::KeyEvent;
use winit::keyboard::KeyCode;

/// The bytes for a winit key event under the program's kitty `flags`: flag 1
/// alone keeps `keykitty::encode`'s forms, anything more is `kittyreport`'s. A release
/// goes out only for a key whose press reached this program (`DOWN`): a crew
/// chord ate that press, and its release must not leak.
pub(crate) fn key_event(
    event: &KeyEvent,
    m: Mods,
    flags: u8,
    broadcast: bool,
    option_as_meta: bool,
) -> Option<Vec<u8>> {
    use crate::kittyreport::{Ev, Kind};
    use winit::keyboard::{KeyLocation, PhysicalKey};
    use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
    let flags = if armed(flags != 0, broadcast) {
        flags
    } else {
        0
    };
    let code = match event.physical_key {
        PhysicalKey::Code(c) => Some(c),
        PhysicalKey::Unidentified(_) => None,
    };
    admit(flags, event.state.is_pressed(), code)
        .then(|| {
            let bare = event.key_without_modifiers();
            let mac = cfg!(target_os = "macos");
            if flags & !1 != 0 {
                let kind = match (event.state.is_pressed(), event.repeat) {
                    (false, _) => Kind::Release,
                    (true, true) => Kind::Repeat,
                    (true, false) => Kind::Press,
                };
                let numpad = event.location == KeyLocation::Numpad;
                let text = event.text.as_deref();
                let e = Ev {
                    logical: &event.logical_key,
                    bare: &bare,
                    text,
                    code,
                    numpad,
                    kind,
                };
                return crate::kittyreport::encode(flags, &e, m, mac, option_as_meta);
            }
            if flags == 0 || !event.state.is_pressed() {
                return None;
            }
            let typed = event.text.as_deref();
            encode(
                &event.logical_key,
                &bare,
                typed,
                m,
                true,
                mac,
                option_as_meta,
            )
        })
        .flatten()
}

/// Whether an event goes on to the encoder: every press, and a release only
/// of a key whose press this program saw (so a chord crew took has no
/// release leaking out). With no flags nothing is kept.
pub(crate) fn admit(flags: u8, pressed: bool, code: Option<KeyCode>) -> bool {
    DOWN.with(|d| {
        let mut d = d.borrow_mut();
        match (flags, pressed) {
            (0, _) => d.clear(),
            (_, true) => d.extend(code),
            (_, false) => return code.is_some_and(|c| d.remove(&c)),
        }
        true
    })
}

thread_local! {
    /// The physical keys pressed while a kitty-flagged program held focus.
    static DOWN: std::cell::RefCell<std::collections::HashSet<KeyCode>> =
        std::cell::RefCell::default();
}

#[cfg(test)]
#[path = "kittykey_tests.rs"]
mod tests;
