use super::*;
use crate::cmddefs::COMMANDS;

#[test]
fn expands_agrees_with_options_for_across_every_command() {
    // `expands` is a cheap shortcut for "does options_for(cmd).is_some()" —
    // duplicated so the palette never has to build rows just to answer a
    // bool. The two lists must never drift apart: a command in one but not
    // the other would silently gain or lose its value picker.
    for c in COMMANDS {
        assert_eq!(
            expands(c.name),
            options_for(c.name).is_some(),
            "{} : expands() and options_for() disagree",
            c.name
        );
    }
}
