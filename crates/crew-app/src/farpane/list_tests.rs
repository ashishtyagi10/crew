use super::read_dir;

#[test]
fn lists_parent_first_then_dirs_then_files() {
    let base = std::env::temp_dir().join("crew_far_list_test");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("zdir")).unwrap();
    std::fs::create_dir_all(base.join("adir")).unwrap();
    std::fs::write(base.join("bfile.txt"), b"x").unwrap();
    let e = read_dir(&base);
    assert!(e[0].is_parent && e[0].name == "..");
    // directories sort before the file, alphabetically
    assert_eq!(e[1].name, "adir");
    assert_eq!(e[2].name, "zdir");
    assert!(e[1].is_dir && !e[3].is_dir);
    assert_eq!(e[3].name, "bfile.txt");
}

#[test]
fn files_sort_by_size_descending_with_name_tiebreak() {
    let base = std::env::temp_dir().join("crew_far_size_sort_test");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("dir")).unwrap();
    std::fs::write(base.join("small.txt"), b"x").unwrap();
    std::fs::write(base.join("big.txt"), vec![b'x'; 500]).unwrap();
    std::fs::write(base.join("also-small.txt"), b"y").unwrap();
    let e = read_dir(&base);
    // ".." then the dir, then files largest-first; equal sizes by name.
    let names: Vec<&str> = e.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        names,
        ["..", "dir", "big.txt", "also-small.txt", "small.txt"]
    );
    assert_eq!(e[2].size, 500);
    assert_eq!(e[1].size, 0, "directories carry no size");
    assert_eq!(e[0].size, 0, "the parent row carries no size");
}
