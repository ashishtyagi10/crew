use super::path_candidates;

/// A unique tempdir per test with a folder, a file, a dotfile, a dotfolder
/// and a folder whose name has a space.
fn fixture(key: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("crew_far_pathcomp_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("src")).unwrap();
    std::fs::create_dir_all(base.join("My Docs")).unwrap();
    std::fs::create_dir_all(base.join(".git")).unwrap();
    std::fs::write(base.join("src.txt"), b"x").unwrap();
    std::fs::write(base.join(".env"), b"x").unwrap();
    base
}

#[test]
fn cd_offers_folders_only_and_other_commands_offer_everything() {
    let base = fixture("dirsonly");
    assert_eq!(path_candidates("sr", &base, true), vec!["src/".to_string()]);
    assert_eq!(
        path_candidates("sr", &base, false),
        vec!["src/".to_string(), "src.txt".to_string()]
    );
}

#[test]
fn dotfiles_hide_until_the_word_starts_with_a_dot() {
    let base = fixture("dots");
    let plain = path_candidates("", &base, false);
    assert!(!plain.iter().any(|c| c.starts_with('.')), "{plain:?}");
    let dotted = path_candidates(".", &base, false);
    assert_eq!(dotted[0], "../", "the parent leads the dotted list");
    assert!(dotted.contains(&".git/".to_string()), "{dotted:?}");
    assert!(dotted.contains(&".env".to_string()), "{dotted:?}");
}

#[test]
fn dot_dot_and_tilde_finish_to_a_folder() {
    let base = fixture("updir");
    assert_eq!(path_candidates("..", &base, true), vec!["../".to_string()]);
    assert_eq!(path_candidates("~", &base, true), vec!["~/".to_string()]);
}

#[test]
fn a_name_with_a_space_comes_back_escaped_and_completes_deeper() {
    let base = fixture("spaces");
    std::fs::create_dir_all(base.join("My Docs/notes")).unwrap();
    assert_eq!(
        path_candidates("My", &base, true),
        vec!["My\\ Docs/".to_string()]
    );
    // The escaped word is what the user has after the first Tab; the next
    // Tab must read inside the folder, keeping the directory part as typed.
    assert_eq!(
        path_candidates("My\\ Docs/no", &base, true),
        vec!["My\\ Docs/notes/".to_string()]
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_folder_is_a_folder() {
    let base = fixture("symlink");
    std::os::unix::fs::symlink(base.join("src"), base.join("link")).unwrap();
    assert_eq!(
        path_candidates("li", &base, true),
        vec!["link/".to_string()]
    );
}
