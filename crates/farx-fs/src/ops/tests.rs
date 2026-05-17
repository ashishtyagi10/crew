use super::*;
use std::sync::mpsc;

#[test]
fn copy_entry_copies_file_into_destination_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src = src_dir.join("file.txt");
    std::fs::write(&src, "hello").unwrap();

    copy_entry(&src, &dst_dir).unwrap();
    assert_eq!(
        std::fs::read_to_string(dst_dir.join("file.txt")).unwrap(),
        "hello"
    );
}

#[test]
fn copy_entry_copies_directory_recursively() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("srcdir");
    let dst = tmp.path().join("dst");
    std::fs::create_dir_all(src.join("nested")).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(src.join("nested").join("a.txt"), "a").unwrap();

    copy_entry(&src, &dst).unwrap();
    assert!(dst.join("srcdir").join("nested").join("a.txt").exists());
}

#[test]
fn move_entry_moves_file() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    let dst_dir = tmp.path().join("dst");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::create_dir_all(&dst_dir).unwrap();
    let src = src_dir.join("move.txt");
    std::fs::write(&src, "mv").unwrap();

    move_entry(&src, &dst_dir).unwrap();
    assert!(!src.exists());
    assert_eq!(
        std::fs::read_to_string(dst_dir.join("move.txt")).unwrap(),
        "mv"
    );
}

#[test]
fn copy_entries_with_progress_sends_finished_message() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("a.txt");
    let dst = tmp.path().join("dst");
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(&src, "progress").unwrap();
    let (tx, rx) = mpsc::channel();

    copy_entries_with_progress(vec![src], dst, tx);
    let events: Vec<FileProgress> = rx.iter().collect();
    let last = events.last().unwrap();
    assert!(last.finished);
    assert!(last.error.is_none());
    assert_eq!(last.files_done, 1);
}

#[test]
fn move_entries_with_progress_sends_finished_message() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("b.txt");
    let dst = tmp.path().join("dst");
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(&src, "progress").unwrap();
    let (tx, rx) = mpsc::channel();

    move_entries_with_progress(vec![src], dst, tx);
    let events: Vec<FileProgress> = rx.iter().collect();
    let last = events.last().unwrap();
    assert!(last.finished);
    assert!(last.error.is_none());
    assert_eq!(last.files_done, 1);
}
