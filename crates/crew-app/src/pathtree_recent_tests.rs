//! Recency in the ranking: with nothing typed the files viewed last lead,
//! newest first; typed, a recent file wins a tie and never a better tier.
use super::*;

fn names(rows: Vec<Row>) -> Vec<String> {
    rows.into_iter().map(|(p, _)| p).collect()
}

fn list(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Empty query: every path ties, so the two recents lead in the order they
/// were viewed, then the rest as before (shorter first). A recent not among
/// the walked files (outside the root, or deleted) is simply not a row.
#[test]
fn nothing_typed_lists_the_recent_files_first() {
    let f = list(&["a.rs", "src/b.rs", "src/deep/c.rs", "d.rs"]);
    let recent = list(&["src/deep/c.rs", "../elsewhere.rs", "gone.rs", "src/b.rs"]);
    assert_eq!(
        names(best(&f, "", &recent)),
        ["src/deep/c.rs", "src/b.rs", "a.rs", "d.rs"]
    );
    assert_eq!(
        names(best(&f, "", &[])),
        ["a.rs", "d.rs", "src/b.rs", "src/deep/c.rs"]
    );
}

/// Typed: two equal matches (same tier, same span) are ordered by recency,
/// not length — but a recent file in a worse tier stays below.
#[test]
fn recency_breaks_a_tie_and_never_lifts_a_worse_tier() {
    let f = list(&["hit.rs", "src/long/hit.rs", "docs/the-hit-list.md"]);
    let recent = list(&["docs/the-hit-list.md", "src/long/hit.rs"]);
    assert_eq!(
        names(best(&f, "hit", &recent)),
        ["src/long/hit.rs", "hit.rs", "docs/the-hit-list.md"],
        "name-prefix tier first; inside it the recent one; the substring match after"
    );
    assert_eq!(
        names(best(&f, "hit", &[])),
        ["hit.rs", "src/long/hit.rs", "docs/the-hit-list.md"]
    );
}

/// Cmd+P's picker over crew's own tree with nothing typed, after two files
/// were viewed: those two lead, marked `recent`, then the folder's rows.
/// `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew quick_open_recent_shot -- --ignored`
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn quick_open_recent_shot() {
    let _g = crate::app::theme_test_guard();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    crate::viewrecent::note(&root.join("README.md"));
    crate::viewrecent::note(&root.join("crates/crew-app/src/pathtree.rs"));
    let mut items = Vec::new();
    for _ in 0..400 {
        items = crate::pathmenu::rows("/view ", &root).expect("a path command");
        if items.first().is_some_and(|i| i.desc == "recent") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(items[0].label, "crates/crew-app/src/pathtree.rs");
    assert_eq!(items[1].label, "README.md");
    let Some(rows) = crate::pickshot_tests::menu_shot("quickopen-recent", "files", &items, 0, 640)
    else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(rows.iter().any(|r| r.contains("recent")), "{rows:?}");
}
