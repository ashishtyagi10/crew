//! The recent-files store: newest first, no repeats, capped, saved where the seam says,
//! and only what still exists under the picker's root offered back.
use super::*;

#[test]
fn a_file_goes_to_the_front_once() {
    let mut l = parse("/a\n/b\n");
    assert!(put_front(&mut l, Path::new("/b")));
    assert_eq!(l, [PathBuf::from("/b"), PathBuf::from("/a")]);
    assert!(!put_front(&mut l, Path::new("/b")), "already newest");
    assert_eq!(l.len(), 2);
}

#[test]
fn the_list_keeps_only_the_newest() {
    let mut l = Vec::new();
    for i in 0..MAX + 5 {
        put_front(&mut l, Path::new(&format!("/f{i}")));
    }
    assert_eq!(l.len(), MAX);
    assert_eq!(l[0], PathBuf::from(format!("/f{}", MAX + 4)));
    let body: String = (0..MAX + 5).map(|i| format!("/g{i}\n/g{i}\n")).collect();
    let read = parse(&body);
    assert_eq!(read.len(), MAX, "a long file reads capped, repeats dropped");
    assert_eq!(read[1], PathBuf::from("/g1"));
}

/// `CREW_VIEW_RECENT_PATH` is where the list lives: a note is written there, and a fresh
/// read (the next launch) gets it back, newest first. Forgetting writes too.
#[test]
fn the_list_survives_a_restart_through_the_seam() {
    let _g = crate::app::theme_test_guard();
    let dir = std::env::temp_dir().join(format!("crew_viewrecent_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (one, two) = (dir.join("one.rs"), dir.join("two.rs"));
    std::fs::write(&one, b"x").unwrap();
    std::fs::write(&two, b"x").unwrap();
    let store = dir.join("store/view-recent");
    std::env::set_var("CREW_VIEW_RECENT_PATH", &store);
    let before = generation();
    note(&one);
    note(&two);
    assert!(generation() > before, "a note bumps the generation");
    *LIST.lock().unwrap() = None; // a new launch: read from disk
    let back = list();
    let at = |p: &Path| {
        back.iter()
            .position(|q| q == &std::fs::canonicalize(p).unwrap())
    };
    assert!(at(&two).unwrap() < at(&one).unwrap(), "{back:?}");
    forget(&two);
    let body = std::fs::read_to_string(&store).unwrap();
    assert!(
        !body.contains("two.rs") && body.contains("one.rs"),
        "{body}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Offered back relative to the root and `/`-separated; a file outside the root, or one
/// deleted since it was viewed, is not offered.
#[test]
fn under_keeps_existing_files_inside_the_root() {
    let _g = crate::app::theme_test_guard();
    let base = std::env::temp_dir().join(format!("crew_viewrecent_under_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("root/src")).unwrap();
    let (a, gone, out) = (
        base.join("root/src/a.rs"),
        base.join("root/gone.rs"),
        base.join("outside.rs"),
    );
    for f in [&a, &gone, &out] {
        std::fs::write(f, b"x").unwrap();
    }
    for f in [&a, &gone, &out] {
        note(f);
    }
    std::fs::remove_file(&gone).unwrap();
    let got = under(&base.join("root"));
    assert_eq!(got, ["src/a.rs"]);
    let _ = std::fs::remove_dir_all(&base);
}
