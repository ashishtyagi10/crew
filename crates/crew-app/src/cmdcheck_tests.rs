use super::*;

/// The on-disk name of the runnable fixture file. Windows has no mode
/// bit — an extension in `PATHEXT` is what makes a file runnable — so the
/// executable is `hit.cmd` there and plain `hit` on Unix. Either way a
/// bare `hit` must resolve, which is what the tests below assert.
const EXE: &str = if cfg!(unix) { "hit" } else { "hit.cmd" };

/// A temp dir holding one executable ([`EXE`]) and one plain, extension-
/// less file `miss` that must NOT resolve on either platform.
fn fixture() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let hit = d.path().join(EXE);
    std::fs::write(&hit, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hit, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::fs::write(d.path().join("miss"), "").unwrap();
    d
}

#[test]
fn first_word_strips_env_prefixes_and_quotes() {
    assert_eq!(first_word("FOO=1 BAR=2 cargo test"), Some("cargo".into()));
    assert_eq!(first_word("\"hit\" --flag"), Some("hit".into()));
    assert_eq!(first_word("  ls -la"), Some("ls".into()));
    assert_eq!(
        first_word("FOO=1"),
        None,
        "only assignments → no command word"
    );
    assert_eq!(first_word(""), None);
}

#[test]
fn resolve_finds_executables_on_the_given_path() {
    let d = fixture();
    let path = d.path().to_str().unwrap().to_string();
    assert_eq!(
        resolve("hit --flag", &path),
        Verdict::Executable("hit".into())
    );
    assert_eq!(resolve("miss", &path), Verdict::No, "non-executable file");
    assert_eq!(resolve("nosuch", &path), Verdict::No);
}

#[test]
fn resolve_accepts_explicit_paths_and_rejects_bad_ones() {
    let d = fixture();
    let hit = d.path().join(EXE);
    assert_eq!(
        resolve(hit.to_str().unwrap(), ""),
        Verdict::Executable(EXE.into()),
        "absolute path bypasses PATH"
    );
    assert_eq!(resolve("./nosuch/prog", ""), Verdict::No);
}

/// A Windows PATH entry carries a drive letter, so the old `split(':')`
/// tore `C:\bin` into `C` and `\bin` and resolved nothing at all. Both
/// platforms are checked here through `join_paths`, which writes the
/// separator the platform actually uses.
#[test]
fn resolve_searches_every_entry_of_a_multi_dir_path() {
    let d = fixture();
    let other = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([other.path(), d.path()])
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert_eq!(
        resolve("hit --flag", &path),
        Verdict::Executable("hit".into()),
        "a later PATH entry still resolves"
    );
    assert_eq!(resolve("nosuch", &path), Verdict::No);
}

#[test]
fn resolve_flags_shell_builtins() {
    assert_eq!(
        resolve("export FOO=1", ""),
        Verdict::Builtin("export".into())
    );
    assert_eq!(
        resolve("source ~/.zshrc", ""),
        Verdict::Builtin("source".into())
    );
}

#[test]
fn effective_path_falls_back_to_process_path() {
    // Hydration hasn't run in tests; must equal the process PATH, not panic.
    assert_eq!(effective_path(), std::env::var("PATH").unwrap_or_default());
}
