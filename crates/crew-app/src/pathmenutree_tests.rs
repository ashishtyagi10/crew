//! The picker's rows from the whole tree ([`crate::pathtree`]), and the
//! commands that must not get them. Split from `pathmenu_tests` for the cap.
use super::rows;
use super::tests::{fixture, labels};

/// Under the folder's rows, files from anywhere in the tree that match — once
/// the background walk has landed — and none repeated from the folder's.
#[test]
fn tree_matches_follow_the_folder_rows() {
    let base = fixture("tree");
    std::fs::create_dir_all(base.join("alpha/deep")).unwrap();
    std::fs::write(base.join("alpha/deep/readme_two.md"), b"x").unwrap();
    let mut got = Vec::new();
    for _ in 0..200 {
        got = labels("/view read", &base);
        if got.len() > 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(got, ["readme.md", "alpha/deep/readme_two.md"]);
    let deep = rows("/view read", &base).unwrap().pop().unwrap();
    assert!(deep.submit, "a file from the tree opens");
    assert_eq!(deep.fill, "/view alpha/deep/readme_two.md");
    // A partial that names a place lists only that place.
    assert!(labels("/view ./read", &base)
        .iter()
        .all(|l| !l.contains("deep")));
}

/// `/dump` writes and `/batch` runs: Enter takes the highlighted row, so a
/// file row there is a file you may never have named. `/dump` lists only
/// folders; neither gets the tree's fuzzy rows.
#[test]
fn writing_and_running_commands_offer_no_file_to_write_over() {
    let base = fixture("write");
    std::fs::create_dir_all(base.join("alpha/deep")).unwrap();
    std::fs::write(base.join("alpha/deep/readme_two.md"), b"x").unwrap();
    // Let a walk of this root land first, so its absence is the rule's.
    for _ in 0..200 {
        if labels("/view read", &base).len() > 1 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        labels("/dump read", &base).is_empty(),
        "no readme.md to overwrite"
    );
    assert_eq!(
        labels("/dump al", &base),
        ["alpha/"],
        "folders to write into"
    );
    assert_eq!(labels("/batch read", &base), ["readme.md"], "no tree rows");
}

/// Cmd+P with nothing typed: the files viewed last lead, newest first and
/// marked `recent`, then the folder's own rows as before — the recent one
/// at the top not listed again among them. `/dump` gets none.
#[test]
fn nothing_typed_leads_with_the_files_viewed_last() {
    let _g = crate::app::theme_test_guard();
    let base = fixture("recent");
    std::fs::create_dir_all(base.join("alpha/deep")).unwrap();
    std::fs::write(base.join("alpha/deep/notes.md"), b"x").unwrap();
    crate::viewrecent::note(&base.join("readme.md"));
    crate::viewrecent::note(&base.join("alpha/deep/notes.md"));
    let mut got = Vec::new();
    for _ in 0..200 {
        got = labels("/view ", &base);
        if got.len() > 4 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        got,
        [
            "alpha/deep/notes.md",
            "readme.md",
            "alpha/",
            "beta/",
            "agenda.txt"
        ]
    );
    let top = rows("/view ", &base).unwrap().remove(0);
    assert_eq!(top.desc, "recent");
    assert_eq!(top.fill, "/view alpha/deep/notes.md");
    assert!(labels("/dump ", &base).iter().all(|l| l.ends_with('/')));
    // Viewing one again reorders a picker already built for this query.
    crate::viewrecent::note(&base.join("readme.md"));
    assert_eq!(
        labels("/view ", &base)[..2],
        ["readme.md", "alpha/deep/notes.md"]
    );
}
