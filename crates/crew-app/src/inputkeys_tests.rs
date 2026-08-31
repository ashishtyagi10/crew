use super::{char_to_insert, delete_last_word};
use winit::keyboard::{Key, NamedKey};

#[test]
fn delete_last_word_cases() {
    let mut s = "ls -la foo".to_string();
    delete_last_word(&mut s);
    assert_eq!(s, "ls -la ");
    let mut s = "one".to_string();
    delete_last_word(&mut s);
    assert_eq!(s, "");
    let mut s = "trailing   ".to_string();
    delete_last_word(&mut s);
    assert_eq!(s, "");
}

#[test]
fn ctrl_held_character_chords_are_swallowed_not_inserted() {
    // Ctrl+O (bound globally to the compact-view toggle in keys.rs) must
    // not leak a literal 'o' into the bar; same for any other unbound
    // ctrl-chord like Ctrl+K.
    assert_eq!(char_to_insert(&Key::Character("o".into()), true), None);
    assert_eq!(char_to_insert(&Key::Character("k".into()), true), None);
}

#[test]
fn plain_characters_still_insert_without_ctrl() {
    assert_eq!(
        char_to_insert(&Key::Character("o".into()), false),
        Some('o')
    );
}

#[test]
fn space_inserts_regardless_of_ctrl() {
    // Space is a Named key, not a Character chord, so it's unaffected by
    // the ctrl gate — unchanged from the pre-existing behavior.
    assert_eq!(
        char_to_insert(&Key::Named(NamedKey::Space), true),
        Some(' ')
    );
    assert_eq!(
        char_to_insert(&Key::Named(NamedKey::Space), false),
        Some(' ')
    );
}

#[test]
fn other_named_keys_do_not_insert() {
    assert_eq!(char_to_insert(&Key::Named(NamedKey::Escape), false), None);
}
