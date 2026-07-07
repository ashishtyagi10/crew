use crate::mdpane::MdPane;
use std::path::PathBuf;

fn pane(source: &str) -> MdPane {
    MdPane::new(
        PathBuf::from("/tmp/mdcache-test-doc.md"),
        source.to_string(),
    )
}

// Finding 2 (Important, phase-2 final review): `cells`/`link_at` each
// independently re-ran `md::render` on the whole file and re-wrapped the raw
// source on every call. Pin the contract via a `#[cfg(test)]` rebuild
// counter: repeated calls at the same width must rebuild the cache once.
#[test]
fn cells_reuses_the_cache_across_repeated_calls_at_the_same_width() {
    let p = pane("hello\nworld");
    let _ = p.cells(41, 5);
    let _ = p.cells(41, 5);
    let _ = p.cells(41, 5);
    assert_eq!(
        p.rebuilds.get(),
        1,
        "repeated calls at the same width must reuse one cache build"
    );
}

#[test]
fn link_at_shares_the_same_cache_cells_builds() {
    let p = pane("hello\nworld");
    let _ = p.cells(41, 5);
    let _ = p.link_at(41, 5, 0, 0);
    assert_eq!(
        p.rebuilds.get(),
        1,
        "link_at at the same width must not trigger its own rebuild"
    );
}

#[test]
fn a_width_change_rebuilds_the_cache() {
    let p = pane("hello\nworld");
    let _ = p.cells(41, 5);
    let _ = p.cells(61, 5);
    assert_eq!(
        p.rebuilds.get(),
        2,
        "a column-width change must invalidate the stale-width cache"
    );
}

#[test]
fn reload_invalidates_the_cache_even_at_the_same_width() {
    let path = std::env::temp_dir().join("crew_mdcache_reload_test.md");
    std::fs::write(&path, "old").unwrap();
    let mut p = MdPane::new(path.clone(), "old".to_string());
    let _ = p.cells(41, 5); // populate the cache with the old content
    assert_eq!(p.rebuilds.get(), 1);
    std::fs::write(&path, "new content").unwrap();
    assert!(p.reload().is_ok());
    let _ = p.cells(41, 5); // same width -- must still rebuild since content changed
    assert_eq!(
        p.rebuilds.get(),
        2,
        "reload must invalidate the cache so stale content isn't served"
    );
    let _ = std::fs::remove_file(&path);
}
