//! A winit key event, adapted for the kitty encoders (`keykitty`'s flag 1,
//! `kittyreport`'s 2/4/8/16), and the memory that lets a release be
//! forwarded only for a press the program saw.
use crate::keyenc::Mods;
use crate::keykitty::{armed, encode};
use crate::kittyheld::{admit, Held};
use winit::event::KeyEvent;

/// The bytes for a winit key event under the program's kitty `flags`: flag 1
/// alone keeps `keykitty::encode`'s forms, anything more is `kittyreport`'s. A release
/// goes out only for a key whose press reached this program (`kittyheld`):
/// a crew chord ate that press, and its release must not leak. `pane` is the
/// target's `born_ms`, kept so a release owed after a focus change finds it.
pub(crate) fn key_event(
    event: &KeyEvent,
    m: Mods,
    flags: u8,
    broadcast: bool,
    option_as_meta: bool,
    pane: u64,
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
    let bare = event.key_without_modifiers();
    let mac = cfg!(target_os = "macos");
    let numpad = event.location == KeyLocation::Numpad;
    let held = || Held {
        pane,
        flags,
        logical: event.logical_key.clone(),
        bare: bare.clone(),
        numpad,
        mods: m,
        mac,
        meta: option_as_meta,
    };
    admit(flags, event.state.is_pressed(), code, held)
        .then(|| {
            if flags & !1 != 0 {
                let kind = match (event.state.is_pressed(), event.repeat) {
                    (false, _) => Kind::Release,
                    (true, true) => Kind::Repeat,
                    (true, false) => Kind::Press,
                };
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
