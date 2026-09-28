use super::*;

/// A fresh directory holding `files`, unique per call.
fn tree(files: &[(&str, &str)]) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-card-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (rel, text) in files {
        let p = d.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    d
}

#[test]
fn a_workspace_is_named_part_by_part_with_each_parts_own_summary() {
    let d = tree(&[
        (
            "README.md",
            "# Crew\n\n[![ci](x)](y)\n\nA native **GPU terminal** in `Rust`,\nsee [the guide](docs/G.md).\n\n## Install\n",
        ),
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\n    \"crates/term\",\n    \"tools/*\",\n]\n",
        ),
        ("crates/term/Cargo.toml", "[package]\nname = \"term\"\n"),
        ("crates/term/src/lib.rs", "//! # term\n//!\n//! terminal model + PTY.\nuse x;\n"),
        ("tools/gen/Cargo.toml", "[package]\ndescription = \"code generator\"\n"),
        (".hidden", ""),
        ("target/x", ""),
    ]);
    let card = card_at(&d).unwrap();
    assert!(
        card.starts_with("PROJECT (the repository you are working in"),
        "{card}"
    );
    assert!(
        card.contains("A native GPU terminal in Rust, see the guide."),
        "{card}"
    );
    assert!(
        card.contains("- crates/term \u{2014} terminal model + PTY."),
        "{card}"
    );
    assert!(
        card.contains("- tools/gen \u{2014} code generator"),
        "{card}"
    );
    assert!(
        card.contains("top level: Cargo.toml, README.md, crates/, tools/"),
        "{card}"
    );
    assert!(
        !card.contains(".hidden") && !card.contains("target/"),
        "{card}"
    );
    assert!(
        !card.contains("Install"),
        "only the opening paragraph: {card}"
    );
}

#[test]
fn an_empty_directory_has_no_card() {
    let d = tree(&[]);
    assert_eq!(card_at(&d), None);
}

#[test]
fn a_huge_readme_is_clipped_and_the_card_stays_bounded() {
    let long = format!("# X\n\n{}\n", "word ".repeat(2_000));
    let many: String = (0..60).map(|i| format!("\"m{i}\",")).collect();
    let d = tree(&[
        ("README.md", &long),
        ("Cargo.toml", &format!("members = [{many}]")),
    ]);
    let card = card_at(&d).unwrap();
    let body = card.split_once(":\n").unwrap().1;
    assert!(body.chars().count() <= CAP + 1, "{}", body.chars().count());
}

/// The card is gathered from the real crew repo without a subprocess and
/// names the crate a live run described as a web framework.
#[test]
fn the_crew_repo_card_names_its_crates() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let card = card_at(&root).unwrap();
    assert!(
        card.contains("crates/crew-term \u{2014} crew-term: terminal model"),
        "{card}"
    );
    // A wrapped doc sentence is read whole, not cut at its first line break.
    assert!(!card.contains("every UI colour, and\n"), "{card}");
    assert!(card.contains("crates/crew-render \u{2014}"), "{card}");
}
