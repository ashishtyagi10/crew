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
    assert!(text.contains("2 languages \u{b7} 1 installed"), "{text}");
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

/// One long server command no longer pads every row past a tile.
#[test]
fn a_long_command_is_cut_in_the_middle_and_the_table_fits_a_tile() {
    let rows: Vec<super::Row> = vec![
        (
            "typescript".into(),
            "/Users/x/.nvm/versions/node/v22.1.0/bin/typescript-language-server --stdio".into(),
            true,
        ),
        ("rust".into(), "rust-analyzer".into(), false),
    ];
    let text = super::listing(&rows, &[]);
    let table: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("  ") && l.contains("installed"))
        .collect();
    assert_eq!(table.len(), 2, "{text}");
    for l in &table {
        assert!(l.chars().count() <= crate::toolsrow::ROW_W, "{l:?}");
    }
    assert!(
        table[0].contains('\u{2026}') && table[0].contains("--stdio"),
        "{:?}",
        table[0]
    );
    assert!(table[1].contains("rust-analyzer"), "{:?}", table[1]);
}
