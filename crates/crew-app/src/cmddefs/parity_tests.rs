//! Every command the palette offers is a command the manuals name.
//!
//! The keybindings already had this contract (`help_tests`'s
//! `the_overlay_and_the_manual_list_the_same_chords`); the commands did not,
//! and they had drifted. README.md — the front door, and the only page most
//! people read — was missing **twelve** shipped commands: `/dash`, `/disk`,
//! `/usage`, `/log`, `/focus`, `/opacity`, `/density`, `/motion`,
//! `/contrast`, `/shapes`, `/weight` and `/smith`. Shipping a feature into a
//! list nobody updated is how a feature stays invisible after it exists.
use super::commands;

fn doc(rel: &str) -> Option<String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(path).ok()
}

/// A command's name appears verbatim in the text, inside backticks — the way
/// both manuals write one.
fn names(text: &str) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    for chunk in text.split('`').skip(1).step_by(2) {
        let word: String = chunk
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '/')
            .collect();
        if word.starts_with('/') && word.len() > 1 {
            out.insert(word);
        }
    }
    out
}

#[test]
fn every_command_is_in_the_manual() {
    let Some(docs) = doc("../../docs/CREW.md") else {
        return; // docs not shipped in this build context
    };
    let listed = names(&docs);
    let missing: Vec<&str> = commands()
        .map(|c| c.name)
        .filter(|n| !listed.contains(*n))
        .collect();
    assert!(
        missing.is_empty(),
        "the palette offers commands docs/CREW.md never mentions: {missing:?}"
    );
}

#[test]
fn every_command_is_in_the_readme() {
    let Some(readme) = doc("../../README.md") else {
        return;
    };
    let listed = names(&readme);
    let missing: Vec<&str> = commands()
        .map(|c| c.name)
        .filter(|n| !listed.contains(*n))
        .collect();
    assert!(
        missing.is_empty(),
        "the palette offers commands README.md never mentions: {missing:?}"
    );
}

/// …and the reverse, so a command that is renamed or dropped does not leave
/// the manuals advertising something neither palette will complete.
///
/// Crew has two: the input bar's (`cmddefs`) and the agent composer's
/// (`chatpaletteitems`, where `/doctor`, `/export` and `/reload` live). A
/// name in the README has to be offered by one of them.
#[test]
fn the_readme_advertises_nothing_either_palette_lacks() {
    let Some(readme) = doc("../../README.md") else {
        return;
    };
    let mut real: std::collections::BTreeSet<&str> = commands().map(|c| c.name).collect();
    real.extend(
        crate::chatpalette::chatpaletteitems::SECTIONS
            .iter()
            .flat_map(|(_, items)| items.iter().copied()),
    );
    // Names the dispatcher still answers to but no palette advertises, each
    // for a reason the README explains where it uses them: `/crew` is
    // `/smith`'s older spelling (both run), `/m` is the composer's one-letter
    // `/model`, and `/restart` is retired — it answers by saying it was
    // merged into `/update`, which is exactly the sentence the README is
    // quoting it in.
    real.extend(["/crew", "/m", "/restart"]);
    let stale: Vec<String> = names(&readme)
        .into_iter()
        .filter(|n| !real.contains(n.as_str()))
        .collect();
    assert!(
        stale.is_empty(),
        "README.md advertises commands no palette offers: {stale:?}"
    );
}
