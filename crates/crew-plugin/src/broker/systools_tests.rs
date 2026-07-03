use super::*;

#[test]
fn enabled_from_defaults_on_and_respects_gates() {
    assert!(enabled_from(None, false));
    assert!(enabled_from(Some("1"), false));
    assert!(!enabled_from(Some("0"), false), "CREW_SYS_TOOLS=0 disables");
    assert!(!enabled_from(None, true), "mock provider disables");
}

#[test]
fn tools_lists_the_four_sys_tools() {
    let t = tools();
    let names: Vec<&str> = t.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["run", "read_file", "write_file", "list_dir"]);
    assert!(t.iter().all(|t| t.server == "sys"));
    assert!(t.iter().all(|t| !t.description.is_empty()));
}

#[test]
fn call_rejects_unknown_tool_and_bad_json() {
    let e = call("nope", "{}").unwrap_err();
    assert!(e.contains("unknown sys tool"), "{e}");
    let e = call("read_file", "not json").unwrap_err();
    assert!(e.contains("not valid JSON"), "{e}");
    let e = call("read_file", "{}").unwrap_err();
    assert!(
        e.contains("missing string argument \u{201c}path\u{201d}"),
        "{e}"
    );
}

#[test]
fn write_then_read_round_trips() {
    let dir = std::env::temp_dir().join(format!("systools-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("note.txt").display().to_string();
    let w = call(
        "write_file",
        &format!(r#"{{"path":{p:?},"content":"hi crew"}}"#),
    )
    .unwrap();
    assert!(w.contains("7 bytes"), "{w}");
    let r = call("read_file", &format!(r#"{{"path":{p:?}}}"#)).unwrap();
    assert_eq!(r, "hi crew");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_errors_are_agent_readable() {
    let e = call("read_file", r#"{"path":"/nonexistent/xyz"}"#).unwrap_err();
    assert!(e.contains("/nonexistent/xyz"), "{e}");
}

#[test]
fn read_file_is_capped() {
    let dir = std::env::temp_dir().join(format!("systools-cap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("big.txt");
    std::fs::write(&p, "x".repeat(CAP + 10)).unwrap();
    let r = call(
        "read_file",
        &format!(r#"{{"path":{:?}}}"#, p.display().to_string()),
    )
    .unwrap();
    assert!(r.len() < CAP + 100, "capped, got {}", r.len());
    assert!(
        r.ends_with("(truncated at 64 KB)"),
        "{}",
        &r[r.len() - 40..]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn list_dir_shows_kind_and_size() {
    let dir = std::env::temp_dir().join(format!("systools-ls-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::write(dir.join("a.txt"), "abc").unwrap();
    let r = call(
        "list_dir",
        &format!(r#"{{"path":{:?}}}"#, dir.display().to_string()),
    )
    .unwrap();
    assert!(r.contains("a.txt (3 B)"), "{r}");
    assert!(r.contains("sub/"), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_truncates_at_utf8_char_boundary() {
    // "é" is 2 bytes (0xC3 0xA9); place it straddling the CAP boundary so the
    // truncation point falls inside the codepoint. The bounded read (File +
    // Read::take) must still walk back to a char boundary and emit valid
    // UTF-8, never a replacement character.
    let dir = std::env::temp_dir().join(format!("systools-utf8b-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("multibyte.txt");
    let mut content = "a".repeat(CAP - 1);
    content.push('é');
    content.push_str(&"b".repeat(100));
    std::fs::write(&p, &content).unwrap();
    let r = call(
        "read_file",
        &format!(r#"{{"path":{:?}}}"#, p.display().to_string()),
    )
    .unwrap();
    assert!(r.len() < CAP + 100, "capped, got {}", r.len());
    assert!(
        r.ends_with("(truncated at 64 KB)"),
        "{}",
        &r[r.len().saturating_sub(40)..]
    );
    assert!(!r.contains('\u{FFFD}'), "no replacement char: {r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_rejects_binary_with_no_boundary_near_cap() {
    // Bytes 0x80..=0xBF are all UTF-8 continuation bytes — none of them is a
    // char boundary. If the walk-back from CAP has no lower bound, it walks
    // past 0 and underflows (`cut -= 1` panics in debug, spins in release).
    // The scan must be bounded to at most 3 steps and, finding no boundary,
    // return the existing agent-readable "not valid UTF-8" error instead.
    let dir = std::env::temp_dir().join(format!("systools-bin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("binary.dat");
    std::fs::write(&p, vec![0x80u8; CAP + 16]).unwrap();
    let e = call(
        "read_file",
        &format!(r#"{{"path":{:?}}}"#, p.display().to_string()),
    )
    .unwrap_err();
    assert!(e.contains("not valid UTF-8"), "{e}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_only_from_recognizes_readonly_and_ro() {
    assert!(read_only_from(Some("readonly")));
    assert!(read_only_from(Some("ro")));
    assert!(!read_only_from(Some("full")));
    assert!(!read_only_from(None));
    assert!(!read_only_from(Some("x")));
}

#[test]
fn read_only_block_gates_mutating_tools_only() {
    assert!(read_only_block("run", true).is_some());
    assert!(read_only_block("write_file", true).is_some());
    assert!(read_only_block("read_file", true).is_none());
    assert!(read_only_block("run", false).is_none());
}

#[test]
fn list_dir_notes_unstatable_entries_instead_of_aborting() {
    let dir = std::env::temp_dir().join(format!("systools-ls-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("a.txt"), "abc").unwrap();
    std::os::unix::fs::symlink("/nonexistent/target", dir.join("dangler")).unwrap();
    let r = call(
        "list_dir",
        &format!(r#"{{"path":{:?}}}"#, dir.display().to_string()),
    )
    .unwrap();
    assert!(r.contains("a.txt (3 B)"), "{r}");
    assert!(r.contains("dangler (?)"), "{r}");
    let _ = std::fs::remove_dir_all(&dir);
}
