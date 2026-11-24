//! The recent-folders list: newest first, no repeats, capped.
use super::*;

#[test]
fn a_folder_goes_to_the_front_once() {
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
        put_front(&mut l, Path::new(&format!("/d{i}")));
    }
    assert_eq!(l.len(), MAX);
    assert_eq!(l[0], PathBuf::from(format!("/d{}", MAX + 4)));
}

#[test]
fn a_file_with_repeats_and_blanks_reads_clean() {
    assert_eq!(
        parse("/a\n\n/a\n /b\n"),
        [PathBuf::from("/a"), PathBuf::from(" /b")]
    );
}
