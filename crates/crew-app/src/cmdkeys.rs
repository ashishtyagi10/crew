//! Which chord runs a command, for the palette's right-hand column.
//!
//! A palette that shows the shortcut beside the command is how anyone stops
//! needing the palette: you read `Cmd+K` on the row you were about to click,
//! and next time you press it. The bindings themselves live in
//! [`crate::keychord`] — this is the mapping from a slash command to the chord
//! that does the same thing, which is a smaller set than either list.
/// `(command, chord)`. A command with no chord is simply absent.
const KEYS: &[(&str, &str)] = &[
    ("/settings", "Cmd+,"),
    ("/sidebar", "Cmd+G"),
    ("/clear", "Cmd+K"),
    ("/zoom", "Cmd+Z"),
    ("/broadcast", "Cmd+S"),
];

/// The chord that runs `cmd`, if one does.
pub(crate) fn key_for(cmd: &str) -> Option<&'static str> {
    KEYS.iter().find(|(name, _)| *name == cmd).map(|(_, k)| *k)
}

#[cfg(test)]
mod tests {
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
        let dispatch = include_str!("chords.rs");
        for (cmd, chord) in KEYS {
            let letter = chord
                .rsplit('+')
                .next()
                .expect("a chord ends in its key")
                .to_lowercase();
            assert!(
                dispatch.contains(&format!("\"{letter}\" =>")),
                "{chord} ({cmd}) is not handled in chords.rs"
            );
        }
    }
}
