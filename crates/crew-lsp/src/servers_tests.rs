use super::*;

#[test]
fn the_builtin_table_names_the_four_ecosystems() {
    let b = builtin();
    assert_eq!(b["rust"].command, "rust-analyzer");
    assert_eq!(b["typescript"].args, ["--stdio"]);
    assert_eq!(b["javascript"], b["typescript"]);
    assert_eq!(b["python"].command, "pyright-langserver");
    assert_eq!(b["go"].command, "gopls");
}

#[test]
fn extensions_map_to_protocol_language_ids() {
    assert_eq!(lang_of(Path::new("a/b.rs")), Some("rust"));
    assert_eq!(lang_of(Path::new("x.tsx")), Some("typescript"));
    assert_eq!(lang_of(Path::new("x.mjs")), Some("javascript"));
    assert_eq!(lang_of(Path::new("x.py")), Some("python"));
    assert_eq!(lang_of(Path::new("x.go")), Some("go"));
    assert_eq!(lang_of(Path::new("x.md")), None);
    assert_eq!(lang_of(Path::new("Makefile")), None);
}

#[test]
fn an_override_file_replaces_by_language_and_adds_new_ones() {
    let user = parse(
        r#"{"servers": {"rust": {"command": "/opt/ra/rust-analyzer", "args": ["--log-file", "/tmp/ra.log"]},
                        "zig": {"command": "zls"}}}"#,
    );
    let all = merged(builtin(), user);
    assert_eq!(all["rust"].command, "/opt/ra/rust-analyzer");
    assert_eq!(all["rust"].args, ["--log-file", "/tmp/ra.log"]);
    assert_eq!(
        all["zig"],
        Server {
            command: "zls".into(),
            args: vec![]
        }
    );
    assert_eq!(all["go"].command, "gopls", "untouched entries survive");
}

#[test]
fn a_malformed_override_is_an_empty_map() {
    assert!(parse("{not json").is_empty());
    assert!(parse(r#"{"servers": 3}"#).is_empty());
    assert!(parse("{}").is_empty());
}

#[test]
fn which_finds_a_program_on_path_and_not_a_made_up_one() {
    let known = if cfg!(windows) { "cmd" } else { "sh" };
    assert!(which(known).is_some_and(|p| p.is_file()));
    assert_eq!(which("crew-no-such-server-xyz"), None);
}

#[test]
fn a_command_with_a_directory_is_checked_where_it_says() {
    let d = std::env::temp_dir().join(format!("crew-lsp-which-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let exe = d.join("fake-ls");
    std::fs::write(&exe, "").unwrap();
    assert_eq!(which(&exe.to_string_lossy()), Some(exe.clone()));
    assert_eq!(which(&d.join("missing").to_string_lossy()), None);
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn a_language_nobody_serves_is_not_available() {
    assert_eq!(available("cobol"), None);
}
