use super::*;
use crate::cmddefs::commands;

/// A shortcut column that names a command the palette does not have is a
/// column that can never be shown — and one that has drifted out of date
/// without anyone noticing.
#[test]
fn every_shortcut_names_a_command_that_exists() {
    for (name, chord) in KEYS {
        assert!(
            commands().any(|c| c.name == *name),
            "{name} ({chord}) is not a command"
        );
    }
}

/// Each command appears once: two chords in one column is a column that
/// silently shows whichever was written first.
#[test]
fn no_command_is_listed_twice() {
    let mut names: Vec<&str> = KEYS.iter().map(|(n, _)| *n).collect();
    names.sort_unstable();
    let before = names.len();
    names.dedup();
    assert_eq!(names.len(), before);
}

#[test]
fn a_command_with_no_chord_has_no_hint() {
    assert_eq!(key_for("/settings"), Some("Cmd+,"));
    assert_eq!(key_for("/dump"), None);
    assert_eq!(key_for(""), None);
}

/// The column claims a chord *does this command*. If the chord stops being
/// handled — or was never handled — the palette teaches a shortcut that
/// does nothing, which is worse than showing no shortcut at all. Read the
/// dispatch itself rather than trusting this table.
#[test]
fn every_chord_is_one_the_dispatch_actually_handles() {
    // Cmd chords are `handle_super_chord`'s arms (`"k" =>`, `"/" | "?" =>`);
    // the Ctrl+Shift walks are tested key by key in `keys.rs`.
    let (cmd_arms, ctrl_keys) = (include_str!("chords.rs"), include_str!("keys.rs"));
    for (cmd, chord) in KEYS {
        let key = chord.rsplit('+').next().expect("a chord ends in its key");
        let handled = match chord.starts_with("Ctrl+") {
            true => {
                ctrl_keys.contains(&format!("eq_ignore_ascii_case(\"{}\")", key.to_lowercase()))
            }
            // `Cmd+Shift+T` is its own shifted arm (`"T" =>`); a plain
            // letter's arm is lower-case.
            false => [key.to_string(), key.to_lowercase()].iter().any(|k| {
                [format!("\"{k}\" =>"), format!("\"{k}\" |")]
                    .iter()
                    .any(|arm| cmd_arms.contains(arm.as_str()))
            }),
        };
        assert!(handled, "{chord} ({cmd}) is not handled");
    }
}
