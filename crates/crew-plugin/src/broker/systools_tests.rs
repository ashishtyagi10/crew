use super::*;
use crate::broker::sysread::PAGE;

#[test]
fn enabled_from_defaults_on_and_respects_gates() {
    assert!(enabled_from(None, false));
    assert!(enabled_from(Some("1"), false));
    assert!(!enabled_from(Some("0"), false), "CREW_SYS_TOOLS=0 disables");
    assert!(!enabled_from(None, true), "mock provider disables");
}

#[test]
fn tools_lists_the_sys_surface() {
    let t = tools();
    let names: Vec<&str> = t.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "run",
            "read_file",
            "write_file",
            "edit",
            "find_tools",
            "search",
            "fetch",
            "grep",
            "glob",
            "outline",
            "git",
            "todo",
            "list_dir"
        ]
    );
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
fn read_file_is_paged() {
    let dir = std::env::temp_dir().join(format!("systools-cap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("big.txt");
    std::fs::write(&p, "x".repeat(CAP + 10)).unwrap();
    let r = call(
        "read_file",
        &format!(r#"{{"path":{:?}}}"#, p.display().to_string()),
    )
    .unwrap();
    assert!(r.len() < PAGE + 200, "one page, got {}", r.len());
    assert!(
        r.contains("(part of line 1 of 1, "),
        "{}",
        &r[r.len() - 90..]
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
    // "é" is 2 bytes (0xC3 0xA9); place it straddling the page end of a line
    // longer than the page, so the cut falls inside the codepoint. The bounded
    // read (File + Read::take) must still walk back to a char boundary and
    // emit valid UTF-8, never a replacement character.
    let dir = std::env::temp_dir().join(format!("systools-utf8b-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("multibyte.txt");
    let mut content = "a".repeat(PAGE - 1);
    content.push('é');
    content.push_str(&"b".repeat(100));
    std::fs::write(&p, &content).unwrap();
    let r = call(
        "read_file",
        &format!(r#"{{"path":{:?}}}"#, p.display().to_string()),
    )
    .unwrap();
    assert!(r.starts_with(&"a".repeat(PAGE - 1)), "{}", &r[PAGE - 5..]);
    assert!(
        r.ends_with(&format!("continue with {{\"offset\": {}}})", PAGE - 1)),
        "{}",
        &r[r.len().saturating_sub(60)..]
    );
    assert!(!r.contains('\u{FFFD}'), "no replacement char: {r}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn read_file_rejects_binary_with_no_boundary_near_the_page_end() {
    // Bytes 0x80..=0xBF are all UTF-8 continuation bytes — none of them is a
    // char boundary. If the walk-back from PAGE has no lower bound, it walks
    // past 0 and underflows (`cut -= 1` panics in debug, spins in release).
    // The scan must be bounded to at most 3 steps and, finding no boundary,
    // return the existing agent-readable "not valid UTF-8" error instead.
    let dir = std::env::temp_dir().join(format!("systools-bin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("binary.dat");
    std::fs::write(&p, vec![0x80u8; PAGE + 16]).unwrap();
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

#[test]
fn read_file_offset_resumes_mid_file() {
    let p = std::env::temp_dir().join(format!("crew-sys-off-{}.txt", std::process::id()));
    std::fs::write(&p, "abcdefghij").unwrap();
    let out = call(
        "read_file",
        &format!("{{\"path\": \"{}\", \"offset\": 4}}", p.display()),
    )
    .unwrap();
    assert_eq!(out, "efghij");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_truncation_notice_names_the_next_offset() {
    let p = std::env::temp_dir().join(format!("crew-sys-big-{}.txt", std::process::id()));
    std::fs::write(&p, "x".repeat(CAP + 100)).unwrap();
    let out = call("read_file", &format!("{{\"path\": \"{}\"}}", p.display())).unwrap();
    assert!(
        out.contains(" of 65,636 \u{2014} "),
        "got tail: {}",
        &out[out.len() - 120..]
    );
    assert!(
        out.ends_with(&format!("continue with {{\"offset\": {PAGE}}})")),
        "got tail: {}",
        &out[out.len() - 120..]
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_offset_past_eof_says_so() {
    let p = std::env::temp_dir().join(format!("crew-sys-eof-{}.txt", std::process::id()));
    std::fs::write(&p, "short").unwrap();
    let out = call(
        "read_file",
        &format!("{{\"path\": \"{}\", \"offset\": 99}}", p.display()),
    )
    .unwrap();
    assert!(out.contains("offset 99"), "got: {out}");
    assert!(out.contains("5 bytes"), "got: {out}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_offset_mid_codepoint_skips_to_a_boundary() {
    let p = std::env::temp_dir().join(format!("crew-sys-utf8-{}.txt", std::process::id()));
    std::fs::write(&p, "é-tail").unwrap(); // 'é' is 2 bytes; offset 1 lands mid-char
    let out = call(
        "read_file",
        &format!("{{\"path\": \"{}\", \"offset\": 1}}", p.display()),
    )
    .unwrap();
    assert_eq!(out, "-tail");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_accepts_a_quoted_numeric_offset() {
    let p = std::env::temp_dir().join(format!("crew-sys-qoff-{}.txt", std::process::id()));
    std::fs::write(&p, "abcdefghij").unwrap();
    let out = call(
        "read_file",
        &format!("{{\"path\": \"{}\", \"offset\": \"4\"}}", p.display()),
    )
    .unwrap();
    assert_eq!(out, "efghij");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_rejects_a_non_numeric_offset() {
    let p = std::env::temp_dir().join(format!("crew-sys-badoff-{}.txt", std::process::id()));
    std::fs::write(&p, "abcdefghij").unwrap();
    let e = call(
        "read_file",
        &format!(
            "{{\"path\": \"{}\", \"offset\": \"not-a-number\"}}",
            p.display()
        ),
    )
    .unwrap_err();
    assert!(e.contains("invalid \u{201c}offset\u{201d}"), "{e}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_at_offset_zero_keeps_strict_utf8_validation() {
    // A file that starts with a lone continuation byte (0x80) is not valid
    // UTF-8 from byte 0. The offset-0 default must not silently skip it and
    // hand back "hi" — it must fail the same way any other invalid-UTF-8
    // file would.
    let p = std::env::temp_dir().join(format!("crew-sys-badstart-{}.dat", std::process::id()));
    std::fs::write(&p, [0x80u8, b'h', b'i']).unwrap();
    let e = call(
        "read_file",
        &format!(r#"{{"path":{:?}}}"#, p.display().to_string()),
    )
    .unwrap_err();
    assert!(e.contains("not valid UTF-8"), "{e}");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn read_file_mid_codepoint_offset_still_reports_truncation() {
    let p = std::env::temp_dir().join(format!("crew-sys-utf8big-{}.txt", std::process::id()));
    let mut content = String::from("é"); // 2 bytes; offset 1 lands mid-char
    content.push_str(&"x".repeat(CAP + 50));
    std::fs::write(&p, &content).unwrap();
    let out = call(
        "read_file",
        &format!("{{\"path\": \"{}\", \"offset\": 1}}", p.display()),
    )
    .unwrap();
    assert!(
        out.starts_with("xxx"),
        "skips the split codepoint, got: {}",
        &out[..20]
    );
    assert!(
        out.contains("(part of line 1 of 1, bytes 2\u{2013}"),
        "tail: {}",
        &out[out.len() - 130..]
    );
    assert!(
        out.contains(&format!("continue with {{\"offset\": {}}}", 1 + 1 + PAGE)),
        "tail: {}",
        &out[out.len() - 130..]
    );
    let _ = std::fs::remove_file(&p);
}

#[test]
fn sys_edit_takes_one_pair_or_a_list_of_them() {
    let dir = std::env::temp_dir().join(format!("crew-systools-edit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.rs");
    std::fs::write(&path, "let a = 1;\nlet b = 2;\n").unwrap();
    let p = path.to_str().unwrap();

    let args = format!(r#"{{"path": "{p}", "old": "let a = 1;", "new": "let a = 9;"}}"#);
    assert!(call("edit", &args).unwrap().contains("at line 1"));

    let args = format!(
        r#"{{"path": "{p}", "edits": [{{"old": "let a = 9;", "new": "let a = 0;"}}, {{"old": "let b = 2;", "new": "let b = 3;"}}]}}"#
    );
    let out = call("edit", &args).unwrap();
    assert_eq!(out.lines().next(), Some("edited a.rs in 2 places"), "{out}");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "let a = 0;\nlet b = 3;\n"
    );

    // A malformed entry names which one, and writes nothing.
    let args = format!(r#"{{"path": "{p}", "edits": [{{"old": "let a = 0;"}}]}}"#);
    let err = call("edit", &args).unwrap_err();
    assert!(err.contains("edits[0]") && err.contains("new"), "{err}");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "let a = 0;\nlet b = 3;\n"
    );
    std::fs::remove_dir_all(&dir).ok();
}

/// A text-mode agent sees only the hint line, clipped at 100 chars, so the
/// checklist's when and its shape both have to fit in it.
#[test]
fn the_todo_line_reaches_a_text_mode_agent_whole() {
    let todo = tools().into_iter().find(|t| t.name == "todo").unwrap();
    let hint = crate::broker::toolcall::hint_for(std::slice::from_ref(&todo));
    assert!(
        hint.contains(&format!("- sys:todo \u{2014} {}\n", todo.description))
            || hint.ends_with(&format!("- sys:todo \u{2014} {}", todo.description)),
        "clipped: {hint}"
    );
    for part in ["3+ steps", "checklist", "\"items\"", "\"status\""] {
        assert!(
            todo.description.contains(part),
            "{part}: {}",
            todo.description
        );
    }
    let e = call("todo", r#"{"items": []}"#).unwrap_err();
    assert!(e.contains("not in one"), "{e}");
}
