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
#[path = "cmdkeys_tests.rs"]
mod tests;
