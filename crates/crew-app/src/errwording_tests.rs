//! A failure reads one way: `could not …`, `cannot …`, `<surface>: cannot
//! write: <why>`.
//!
//! The status line said `couldn't open shell` beside fifteen `could not`s,
//! `can't copy` beside eleven `cannot`s, and one `cannot write {e}` without
//! the colon its six siblings put before the reason.

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

#[test]
fn failures_are_spelled_out() {
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
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            // Only inside string literals: the odd-numbered `"` segments.
            let lits = line.split('"').skip(1).step_by(2);
            if lits
                .clone()
                .any(|l| l.contains("couldn't") || l.contains("can't"))
            {
                bad.push(format!("{rel}:{}: {}", n + 1, code));
            }
            if lits.clone().any(|l| l.contains("cannot write {")) {
                bad.push(format!(
                    "{rel}:{}: {} (colon before the reason)",
                    n + 1,
                    code
                ));
            }
        }
    }
    assert!(bad.is_empty(), "\n{}", bad.join("\n"));
}
