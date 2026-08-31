use super::{complete_path, path_suggest};

/// Per-test fixture dir: a SHARED path raced under parallel runs (one
/// test's remove_dir_all deleted the tree mid-assertion in the other).
fn fixture(name: &str) -> std::path::PathBuf {
    let base =
        std::env::temp_dir().join(format!("crew_pathcomplete_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("alpha")).unwrap();
    std::fs::write(base.join("readme.md"), b"x").unwrap();
    base
}

#[test]
fn completes_dirs_only_or_files_too() {
    let base = fixture("complete");
    // dirs-only: the directory matches (trailing slash), the file does not.
    assert_eq!(complete_path("al", &base, false).as_deref(), Some("pha/"));
    assert_eq!(complete_path("read", &base, false), None);
    // files_too: the file completes (no trailing slash).
    assert_eq!(complete_path("read", &base, true).as_deref(), Some("me.md"));
}

#[test]
fn every_path_command_completes_paths() {
    let base = fixture("suggest");
    for cmd in super::PATH_COMMANDS {
        assert_eq!(
            path_suggest(&format!("{cmd} al"), &base).as_deref(),
            Some("pha/"),
            "{cmd} does not complete a directory"
        );
        assert_eq!(
            path_suggest(&format!("{cmd} read"), &base).as_deref(),
            Some("me.md"),
            "{cmd} does not complete a file"
        );
    }
    // Other commands and a trailing-slash partial complete nothing.
    assert_eq!(path_suggest("/run al", &base), None);
    assert_eq!(path_suggest("/dump alpha/", &base), None);
    // The space is part of the prefix: a command name that merely STARTS
    // with a path command's name is a different command.
    assert_eq!(path_suggest("/viewer al", &base), None);
}

/// The palette's descriptions and the completion list must agree. Each
/// of `/view`, `/md` and `/batch` shipped as a command you type a path
/// into with no completion at all, for as long as it had existed, because
/// nothing compared the two lists.
#[test]
fn every_command_that_takes_a_path_completes_one() {
    for c in crate::cmddefs::commands() {
        let takes_path = c.desc.contains("<path>") || c.desc.contains("<file>");
        assert_eq!(
            takes_path,
            super::PATH_COMMANDS.contains(&c.name),
            "{} says {:?} but is {}in PATH_COMMANDS",
            c.name,
            c.desc,
            if takes_path { "not " } else { "" }
        );
    }
}
