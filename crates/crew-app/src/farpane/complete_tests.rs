use super::*;

fn fixture(key: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("crew_far_complete_{key}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("src")).unwrap();
    std::fs::create_dir_all(base.join("srcdocs")).unwrap();
    std::fs::write(base.join("src/main.rs"), b"x").unwrap();
    std::fs::write(base.join("src/Main.txt"), b"x").unwrap();
    std::fs::write(base.join("readme.md"), b"x").unwrap();
    base
}

#[test]
fn caret_token_splits_command_and_path_words() {
    assert_eq!(caret_token("ls"), (TokenKind::Command, "ls"));
    assert_eq!(caret_token("ls src/fa"), (TokenKind::Path, "src/fa"));
}

#[test]
fn caret_token_trailing_space_starts_an_empty_token() {
    assert_eq!(caret_token("ls "), (TokenKind::Path, ""));
}

#[test]
fn cd_argument_is_always_a_path_token() {
    assert_eq!(caret_token("cd sr"), (TokenKind::Path, "sr"));
}

#[test]
fn path_candidates_list_the_tokens_parent_dir_with_dir_slash_suffix() {
    let base = fixture("pathlist");
    let cands = candidates("ls ", &base, &[]);
    assert!(cands.contains(&"src/".to_string()), "{cands:?}");
    assert!(cands.contains(&"srcdocs/".to_string()), "{cands:?}");
    assert!(cands.contains(&"readme.md".to_string()), "{cands:?}");
}

#[test]
fn path_candidates_prefix_match_case_sensitive_then_insensitive() {
    let base = fixture("pathcase");
    // "M" matches "Main.txt" case-sensitively first, "main.rs" only
    // case-insensitively — case-sensitive matches must come first.
    let cands = candidates("ls src/M", &base, &[]);
    assert_eq!(
        cands,
        vec!["src/Main.txt".to_string(), "src/main.rs".to_string()]
    );
}

#[test]
fn a_unique_prefix_yields_a_single_candidate() {
    let base = fixture("unique");
    let cands = candidates("cat read", &base, &[]);
    assert_eq!(cands, vec!["readme.md".to_string()]);
}

#[test]
fn command_candidates_are_builtins_plus_binaries_prefix_matched() {
    let base = fixture("cmdlist");
    let bins = vec!["cargo".to_string(), "cat".to_string(), "ls".to_string()];
    let cands = candidates("ca", &base, &bins);
    assert_eq!(cands, vec!["cargo".to_string(), "cat".to_string()]);
}

#[test]
fn cd_argument_completes_directories_in_context() {
    let base = fixture("cdpath");
    let cands = candidates("cd sr", &base, &[]);
    assert_eq!(cands, vec!["src/".to_string(), "srcdocs/".to_string()]);
}

#[test]
fn apply_replaces_only_the_caret_token() {
    assert_eq!(apply("ls src/fa", "src/farpane/"), "ls src/farpane/");
    assert_eq!(apply("ca", "cargo"), "cargo");
    assert_eq!(apply("ls ", "readme.md"), "ls readme.md");
}

#[cfg(unix)]
#[test]
fn scan_path_binaries_finds_executables_only_sorted_and_deduped() {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::temp_dir().join("crew_far_complete_pathscan");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let exe = base.join("mytool");
    std::fs::write(&exe, b"#!/bin/sh\n").unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(base.join("notes.txt"), b"x").unwrap();
    // Duplicate PATH entry — the scan must dedupe across dirs too.
    let path_var = format!("{}:{}", base.display(), base.display());
    assert_eq!(scan_path_binaries(&path_var), vec!["mytool".to_string()]);
}
