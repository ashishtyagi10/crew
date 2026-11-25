//! The pinned list: read, written, and held on the Explorer — never read from the real profile
//! under test.
use std::path::PathBuf;

use super::*;

#[test]
fn the_store_drops_blanks_and_repeats_and_keeps_the_order_pinned() {
    let got = parse("/b\n\n/a\n/b\n  \n/c\n");
    let want: Vec<PathBuf> = ["/b", "/a", "/c"].iter().map(PathBuf::from).collect();
    assert_eq!(got, want);
    let many: String = (0..MAX + 5).map(|i| format!("/d{i}\n")).collect();
    assert_eq!(parse(&many).len(), MAX);
}

#[test]
fn what_is_saved_loads_back() {
    let dir = std::env::temp_dir().join(format!("crew-explorer-roots-{}", std::process::id()));
    let p = dir.join("nested").join("explorer-roots");
    let list = vec![PathBuf::from("/x/creds"), PathBuf::from("/y/conf")];
    save_to(&p, &list);
    assert_eq!(load_from(&p), list);
    assert!(load_from(&dir.join("missing")).is_empty());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn an_explorer_starts_with_nothing_pinned_and_pins_each_folder_once() {
    let e = Explorer::default();
    assert!(e.pinned().is_empty(), "a test app never reads the profile");
    let (a, b) = (PathBuf::from("/x/a"), PathBuf::from("/x/b"));
    assert!(e.pin(&a));
    assert!(!e.pin(&a), "already pinned");
    assert!(e.pin(&b));
    assert_eq!(
        e.pinned(),
        vec![a.clone(), b.clone()],
        "in the order pinned"
    );
    assert!(e.unpin(&a));
    assert!(!e.unpin(&a), "gone already");
    assert_eq!(e.pinned(), vec![b]);
}

#[test]
fn a_full_list_pins_nothing_more() {
    let e = Explorer::default();
    for i in 0..MAX {
        assert!(e.pin(&PathBuf::from(format!("/d{i}"))));
    }
    assert!(!e.pin(&PathBuf::from("/one-more")));
    assert_eq!(e.pinned().len(), MAX);
}

#[test]
fn a_pin_is_still_there_for_the_next_explorer() {
    let dir = std::env::temp_dir().join(format!("crew-explorer-keep-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let store = dir.join("explorer-roots");
    TEST_PATH.with(|p| *p.borrow_mut() = Some(store.clone()));
    let creds = PathBuf::from("/x/creds");
    assert!(Explorer::default().pin(&creds));
    assert_eq!(
        Explorer::default().pinned(),
        vec![creds.clone()],
        "read back"
    );
    assert!(Explorer::default().unpin(&creds));
    assert!(
        Explorer::default().pinned().is_empty(),
        "the unpin was saved too"
    );
    TEST_PATH.with(|p| *p.borrow_mut() = None);
    let _ = std::fs::remove_dir_all(dir);
}
