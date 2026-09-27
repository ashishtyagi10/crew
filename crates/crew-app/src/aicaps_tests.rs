//! "AI", in capitals, wherever crew says it.
//!
//! The `?` prefix said `ask ai for a command` and `asking ai…` while `/keys`
//! and the broker's `/help` said "Ask the AI". An initialism is capitals.

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// The word `ai`, lowercase, inside a string literal on this line.
fn lowercase_ai_in_a_string(line: &str) -> bool {
    line.split('"').skip(1).step_by(2).any(|lit| {
        let b = lit.as_bytes();
        lit.match_indices("ai").any(|(i, _)| {
            let edge = |c: Option<&u8>| {
                c.is_none_or(|c| !c.is_ascii_alphanumeric() && *c != b'_' && *c != b'{')
            };
            edge(i.checked_sub(1).and_then(|j| b.get(j))) && edge(b.get(i + 2))
        })
    })
}

#[test]
fn ai_is_written_in_capitals() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&src, &mut files);
    let mut bad = Vec::new();
    for p in files {
        let rel = p
            .strip_prefix(&src)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel.ends_with("_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        for (n, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") && lowercase_ai_in_a_string(line) {
                bad.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(bad.is_empty(), "write AI:\n{}", bad.join("\n"));
}

#[test]
fn the_scan_finds_the_word_not_the_letters() {
    assert!(lowercase_ai_in_a_string(
        r#"set_status("ask ai for a command")"#
    ));
    assert!(!lowercase_ai_in_a_string(
        r#"set_status("ask AI for a command")"#
    ));
    assert!(!lowercase_ai_in_a_string(
        r#"set_status("waiting on the chain")"#
    ));
    assert!(!lowercase_ai_in_a_string(r#"format!("{ai}")"#));
}
