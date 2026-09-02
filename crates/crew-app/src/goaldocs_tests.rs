use super::*;

/// The changelog's version headings, single (`## 0.21.16`) or a range
/// (`## 0.6.38 – 0.6.61`), as `(from, to)` triples.
type Triple = (u64, u64, u64);

fn changelog_headings() -> Vec<(Triple, Triple)> {
    let text = std::fs::read_to_string(root().join("CHANGELOG.md")).unwrap();
    text.lines()
        .filter_map(|l| l.strip_prefix("## "))
        .filter_map(|h| {
            let mut ends = h.split(['\u{2013}', '-']).map(str::trim).filter_map(triple);
            let from = ends.next()?;
            Some((from, ends.next().unwrap_or(from)))
        })
        .collect()
}

fn triple(v: &str) -> Option<Triple> {
    let mut it = v
        .split_whitespace()
        .next()?
        .split('.')
        .map(str::parse::<u64>);
    Some((it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?))
}

/// Every version a goal document says something shipped in has a changelog
/// entry — the one place a release is written down.
#[test]
fn every_version_a_goal_cites_is_in_the_changelog() {
    let headings = changelog_headings();
    assert!(
        headings.len() > 100,
        "the headings parse found {}",
        headings.len()
    );
    let mut seen = 0;
    for (name, text) in goal_docs() {
        for v in cited_versions(&text) {
            seen += 1;
            let t = triple(&v).unwrap();
            assert!(
                headings.iter().any(|(from, to)| *from <= t && t <= *to),
                "{name} cites v{v}, and CHANGELOG.md has no entry for it"
            );
        }
    }
    assert!(seen >= 40, "the parse found only {seen} cited versions");
}

/// Every source file a goal document points at exists. A goal that says
/// "the map lives in `docwin/keys.rs`" and a tree where it does not is a
/// document sending a reader to the wrong place — and it is exactly the
/// kind of drift a status line rides on.
#[test]
fn every_source_a_goal_cites_exists() {
    let files = source_files();
    assert!(files.len() > 500, "the walk found {} files", files.len());
    let mut seen = 0;
    let mut missing = Vec::new();
    for (name, text) in goal_docs() {
        for cited in cited_sources(&text) {
            seen += 1;
            if !resolves(&cited, &files) {
                missing.push(format!("{name} cites `{cited}`"));
            }
        }
    }
    assert!(seen >= 100, "the parse found only {seen} cited files");
    assert!(
        missing.is_empty(),
        "no such file under crates/ or vendor/ \u{2014} the document has drifted from \
         the tree; point it at where the code went:\n  {}",
        missing.join("\n  ")
    );
}

/// The parses read what they claim to: a version inside a word (`crew-v1`,
/// `v0.21.16-rc`) is not a citation, a path's line suffix is dropped, and a
/// backticked command that merely ends in `.rs` is not a file.
#[test]
fn the_parses_are_not_fooled() {
    assert_eq!(
        cited_versions("shipped v0.20.4 and v0.20.5."),
        ["0.20.4", "0.20.5"]
    );
    assert!(cited_versions("nav1.2.3 and v1.2").is_empty());
    assert_eq!(
        cited_sources("in `broker/session.rs:255`, `keys.rs`, `.rs` and `cargo test -- x.rs`"),
        ["broker/session.rs", "keys.rs"]
    );
    let files = vec!["crates/a/src/docwin/keys.rs".to_string()];
    assert!(resolves("keys.rs", &files));
    assert!(resolves("docwin/keys.rs", &files));
    assert!(!resolves("win/keys.rs", &files), "a tail lands on a `/`");
}
