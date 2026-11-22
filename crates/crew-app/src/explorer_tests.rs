//! The tree's model: what a folder lists, and how the tree flattens.
use super::*;

/// A scratch directory holding `files` (a trailing `/` makes a folder).
pub(crate) fn tree(name: &str, files: &[&str]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("crew-explorer-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for f in files {
        let p = root.join(f.trim_end_matches('/'));
        match f.ends_with('/') {
            true => std::fs::create_dir_all(&p).unwrap(),
            false => {
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                std::fs::write(&p, "x").unwrap();
            }
        }
    }
    root
}

fn names(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .map(|i| format!("{}{}", "  ".repeat(i.depth.into()), i.name))
        .collect()
}

#[test]
fn folders_come_first_then_files_by_name_ignoring_case() {
    let root = tree(
        "sort",
        &[
            "b.txt",
            "A.txt",
            "zdir/",
            "Cdir/",
            ".git/",
            ".DS_Store",
            ".env",
        ],
    );
    let (v, more) = read(&root);
    let got: Vec<&str> = v.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        got,
        ["Cdir", "zdir", ".env", "A.txt", "b.txt"],
        "and .git / .DS_Store hidden"
    );
    assert_eq!(more, 0);
}

#[test]
fn the_root_starts_open_and_a_folder_opens_on_toggle() {
    let root = tree("toggle", &["src/main.rs", "src/lib.rs", "README.md"]);
    let mut e = Explorer::default();
    let items = e.items(&root);
    assert_eq!(names(&items)[1..], ["  src", "  README.md"]);
    e.toggle(&items[1]);
    let open = e.items(&root);
    assert_eq!(
        names(&open)[1..],
        ["  src", "    lib.rs", "    main.rs", "  README.md"]
    );
    // The root closes too — to its own row.
    e.toggle(&open[0]);
    assert_eq!(e.items(&root).len(), 1);
}

/// A file written after the folder was listed appears once the listing goes stale.
#[test]
fn a_new_file_appears_once_the_listing_is_stale() {
    let root = tree("stale", &["a.txt"]);
    let e = Explorer::default();
    assert_eq!(e.items(&root).len(), 2);
    std::fs::write(root.join("b.txt"), "x").unwrap();
    e.listings.borrow_mut().get_mut(&root).unwrap().at -= STALE;
    // Stale: the old listing is drawn while a worker reads the new one…
    assert_eq!(names(&e.items(&root))[1..], ["  a.txt"]);
    // …which lands on a later frame.
    for _ in 0..200 {
        if e.items(&root).len() == 3 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(names(&e.items(&root))[1..], ["  a.txt", "  b.txt"]);
}

#[test]
fn a_folder_past_the_cap_says_how_many_it_left_out() {
    let files: Vec<String> = (0..CAP + 3).map(|i| format!("f{i:05}")).collect();
    let refs: Vec<&str> = files.iter().map(String::as_str).collect();
    let root = tree("cap", &refs);
    let items = Explorer::default().items(&root);
    assert_eq!(items.len(), 1 + CAP + 1);
    assert_eq!(items.last().unwrap().more, Some(3));
}

#[test]
fn scrolling_stays_inside_the_list() {
    let mut e = Explorer::default();
    e.scroll_by(-5, 20, 10);
    assert_eq!(e.scroll, 5);
    e.scroll_by(-50, 20, 10);
    assert_eq!(e.scroll, 10, "no further than the last page");
    e.scroll_by(50, 20, 10);
    assert_eq!(e.scroll, 0);
}
