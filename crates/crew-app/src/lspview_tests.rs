use super::*;

fn rows() -> Vec<Row> {
    vec![
        ("go".into(), "gopls".into(), false),
        ("rust".into(), "rust-analyzer".into(), true),
    ]
}

/// The command is in the palette — the only way anyone finds it.
#[test]
fn the_command_is_registered() {
    assert!(crate::cmddefs::commands().any(|c| c.name == "/lsp"));
}

/// A server that is not there is said to be not there, by name, on its
/// own row — the whole point of the card.
#[test]
fn an_uninstalled_server_is_named_not_installed() {
    let text = listing(&rows(), &[]);
    let go = text
        .lines()
        .find(|l| l.contains("gopls"))
        .expect("a go row");
    assert!(go.ends_with("not installed"), "{go}");
    let rust = text.lines().find(|l| l.contains("rust-analyzer")).unwrap();
    assert!(rust.ends_with("  installed"), "{rust}");
    assert!(text.contains("2 language(s) \u{b7} 1 installed"), "{text}");
    assert!(text.contains("no server running"), "{text}");
}

#[test]
fn running_roots_are_listed_under_their_language() {
    let text = listing(&rows(), &[("rust".into(), PathBuf::from("/p/crew"))]);
    assert!(
        text.contains("running in this crew:\n  rust  /p/crew"),
        "{text}"
    );
    assert!(!text.contains("no server running"));
}

/// Columns line up: the language and command columns are padded to the
/// widest entry, so the state reads as a column too.
#[test]
fn the_state_column_is_aligned() {
    let text = listing(&rows(), &[]);
    let at: Vec<usize> = text
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("installed"))
        .map(|l| l.find("installed").unwrap() - if l.contains("not ") { 4 } else { 0 })
        .collect();
    assert_eq!(at.len(), 2);
    assert_eq!(at[0], at[1], "{text}");
}
