//! The command palette's answer when nothing matches.
//!
//! Type `/xyz` and the palette used to disappear — a card that vanishes
//! reads as a key that did nothing, and it is indistinguishable from a
//! rendering fault. The `/keys` overlay states the rule already ("an empty
//! panel reads as a rendering fault") and answers with one dim note; this is
//! the same note for the two palettes above the input bar: the command list,
//! and a closed-set value picker whose values the argument has filtered to
//! nothing. A freeform argument (`/run cargo …`) has no list to be empty, so
//! it gets no note.
//!
//! The note is a HEADER row — never selected, never filled, never run. The
//! bar's keys look for a selectable row before they treat the palette as
//! open, so with only a note showing Up/Down still recall history, Tab still
//! accepts the ghost, and Enter still submits the line to dispatch, which is
//! where "unknown command — did you mean" lives.
use crate::suggest::MenuItem;

/// The palette rows for `text`: what [`crate::suggest::menu_items_in`] finds,
/// or one note saying nothing was found.
pub(crate) fn rows(text: &str, cwd: &std::path::Path) -> Vec<MenuItem> {
    let items = crate::suggest::menu_items_in(text, cwd);
    if !items.is_empty() {
        return items;
    }
    note_for(text, cwd).into_iter().collect()
}

/// The note for `text`, when `text` asked a list a question and got nothing.
fn note_for(text: &str, cwd: &std::path::Path) -> Option<MenuItem> {
    let rest = text.strip_prefix('/')?;
    if rest.is_empty() {
        return None;
    }
    let words = match rest.split_once(' ') {
        None => format!("no command matches \"/{rest}\" \u{b7} /help"),
        Some((cmd, arg)) => {
            // A picker with a closed set lists it on a bare `/cmd `; a
            // freeform argument lists nothing there either, and is not a
            // miss.
            let closed = !crate::suggest::menu_items_in(&format!("/{cmd} "), cwd).is_empty();
            if !closed {
                return None;
            }
            format!("no /{cmd} value matches \"{}\"", arg.trim())
        }
    };
    Some(MenuItem {
        label: words,
        header: true,
        ..Default::default()
    })
}

/// Whether `items` has a row the keys can land on. A note-only palette is
/// drawn but not OPEN: nothing to step to, nothing to fill, nothing to run.
pub(crate) fn selectable(items: &[MenuItem]) -> bool {
    items.iter().any(|i| !i.header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn a_miss_is_one_note_and_a_hit_is_untouched() {
        let cwd = Path::new("");
        let miss = rows("/xyzzy", cwd);
        assert_eq!(miss.len(), 1);
        assert!(miss[0].header, "a note is a header: never selected");
        assert!(miss[0].label.contains("no command matches \"/xyzzy\""));
        assert!(miss[0].label.contains("/help"), "{}", miss[0].label);
        assert!(!selectable(&miss));
        let hit = rows("/the", cwd);
        assert!(hit.iter().any(|i| i.label == "/theme"));
        assert!(selectable(&hit));
        assert_eq!(hit.len(), crate::suggest::menu_items_in("/the", cwd).len());
    }

    #[test]
    fn a_closed_picker_notes_its_miss_and_a_freeform_arg_does_not() {
        let cwd = Path::new("");
        let miss = rows("/theme wobble", cwd);
        assert_eq!(miss.len(), 1);
        assert_eq!(miss[0].label, "no /theme value matches \"wobble\"");
        // `/run <anything>` has no list, so there is nothing to be empty.
        assert!(rows("/run wobble", cwd).is_empty());
        // The bare slash lists every command; not a miss.
        assert!(selectable(&rows("/", cwd)));
    }
}
