use super::*;
use crate::types::{SortField, SortOrder};

fn setup_tree_root() -> (tempfile::TempDir, TreeState) {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("alpha.txt"), "a").unwrap();
    std::fs::write(tmp.path().join("big.bin"), vec![0u8; 64]).unwrap();
    std::fs::create_dir(tmp.path().join("dir")).unwrap();
    std::fs::write(tmp.path().join("dir").join("nested.txt"), "nested").unwrap();

    let tree = TreeState::new(tmp.path().to_path_buf());
    (tmp, tree)
}

#[test]
fn toggle_select_skips_parent_entry() {
    let (_tmp, mut tree) = setup_tree_root();
    if tree.visible_nodes.first().map(|n| n.entry.name.as_str()) == Some("..") {
        tree.cursor = 0;
        tree.toggle_select();
        assert!(tree.selected.is_empty());
        assert_eq!(tree.cursor, 1);
    }
}

#[test]
fn navigate_history_back_and_forward() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("a");
    let second = tmp.path().join("b");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();

    let mut tree = TreeState::new(first.clone());
    tree.navigate_to(second.clone());
    assert_eq!(tree.root, second);
    assert_eq!(tree.history_back.len(), 1);

    assert!(tree.go_back());
    assert_eq!(tree.root, first);
    assert!(tree.go_forward());
    assert_eq!(tree.root, second);
}

#[test]
fn filter_keeps_directories_visible() {
    let (_tmp, mut tree) = setup_tree_root();
    tree.filter = "alpha".to_string();
    tree.rebuild();

    let names: Vec<String> = tree
        .visible_nodes
        .iter()
        .map(|n| n.entry.name.clone())
        .collect();
    assert!(names.iter().any(|n| n == "alpha.txt"));
    assert!(names.iter().any(|n| n == "dir"));
    assert!(!names.iter().any(|n| n == "big.bin"));
}

#[test]
fn expand_then_collapse_moves_cursor_to_parent() {
    let (_tmp, mut tree) = setup_tree_root();
    let dir_index = tree
        .visible_nodes
        .iter()
        .position(|n| n.entry.name == "dir")
        .unwrap();
    tree.cursor = dir_index;
    tree.expand();
    assert!(tree.visible_nodes[tree.cursor].depth > tree.visible_nodes[dir_index].depth);
    let parent_index = tree
        .visible_nodes
        .iter()
        .position(|n| n.entry.name == "dir")
        .unwrap();
    tree.collapse();
    assert_eq!(tree.cursor, parent_index);
}

#[test]
fn sort_by_size_descending_reorders_files() {
    let (_tmp, mut tree) = setup_tree_root();
    tree.sort_field = SortField::Size;
    tree.sort_order = SortOrder::Descending;
    tree.rebuild();

    let files: Vec<String> = tree
        .visible_nodes
        .iter()
        .filter(|n| n.depth == 0 && !n.entry.is_dir && n.entry.name != "..")
        .map(|n| n.entry.name.clone())
        .collect();
    assert_eq!(files.first().map(String::as_str), Some("big.bin"));
}
