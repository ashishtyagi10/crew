use super::*;
use std::path::{Path, PathBuf};

fn item(path: &str, dir: bool, depth: u16) -> Item {
    Item {
        depth,
        name: Path::new(path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into(),
        path: PathBuf::from(path),
        dir,
        open: false,
        more: None,
    }
}

const OUT: &str = "## main\n M src/a.rs\n?? notes.md\n";

#[test]
fn files_folders_and_the_root() {
    let g = crate::gitfiles::parse(OUT, Path::new("/r"));
    assert_eq!(
        said(&g, &item("/r/src/a.rs", false, 2)),
        Some(Said::File(Mark::Modified))
    );
    assert_eq!(
        said(&g, &item("/r/notes.md", false, 1)),
        Some(Said::File(Mark::Untracked))
    );
    assert_eq!(said(&g, &item("/r/src", true, 1)), Some(Said::Holds));
    assert_eq!(said(&g, &item("/r/docs", true, 1)), None);
    assert_eq!(said(&g, &item("/r/src/b.rs", false, 2)), None);
    assert_eq!(
        said(&g, &item("/r", true, 0)),
        None,
        "the root is the whole repository"
    );
}

/// In the tree: the changed file's letter at the row's end in the change's ink, the folder's
/// dot, and nothing on a clean row.
#[test]
fn the_tree_draws_the_marks() {
    let _g = crate::app::theme_test_guard();
    let g = crate::gitfiles::parse(OUT, Path::new("/r"));
    let items = vec![
        item("/r", true, 0),
        item("/r/docs", true, 1),
        item("/r/src", true, 1),
        item("/r/src/a.rs", false, 2),
    ];
    let marks = crate::explorerdraw::Marks {
        git: &[&g],
        ..Default::default()
    };
    let cells = crate::explorerdraw::cells(&items, 0, marks, (30, 10));
    let at = |row: u16| {
        cells
            .iter()
            .find(|c| c.row == crate::explorerdraw::TOP + row && c.col == 29)
            .map(|c| (c.c, c.fg))
    };
    let amber = crew_theme::readable::warn(crew_theme::theme());
    assert_eq!(at(3), Some(('M', amber)));
    assert_eq!(at(2), Some(('\u{2022}', amber)));
    assert_eq!(at(1), None, "docs is clean");
    assert_eq!(at(0), None, "the root carries no mark");
}
