use super::*;
use crate::broker::systools;

fn dir(files: &[(&str, &str)]) -> std::path::PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("crew-outline-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    for (rel, text) in files {
        std::fs::write(d.join(rel), text).unwrap();
    }
    d
}

#[test]
fn a_language_it_does_not_know_is_sent_to_grep() {
    assert_eq!(
        render("Cargo.toml", "[package]\nname = \"x\"\n").unwrap_err(),
        "no outline for .toml files \u{2014} use sys:grep to find a name in it"
    );
    let e = render("Makefile", "all:\n").unwrap_err();
    assert!(
        e.starts_with("no outline for Makefile, a file with no extension"),
        "{e}"
    );
    assert!(e.ends_with("use sys:grep to find a name in it"), "{e}");
}

/// The acceptance case, on a file this repo really has: the row for
/// `pub fn frame` carries the line `grep -n` gives it.
#[test]
fn route_rs_outlines_fn_frame_at_the_line_grep_finds_it() {
    let path = "src/broker/route.rs";
    let src = std::fs::read_to_string(path).unwrap();
    let want = src
        .lines()
        .position(|l| l.contains("pub fn frame"))
        .expect("route.rs has a pub fn frame")
        + 1;
    let out = outline(path).unwrap();
    assert!(
        out.starts_with(&format!(
            "route.rs \u{2014} {} lines\n",
            src.lines().count()
        )),
        "{out}"
    );
    let row = out
        .lines()
        .find(|r| r.contains("pub fn frame("))
        .unwrap_or_else(|| panic!("no pub fn frame row in\n{out}"));
    let at: usize = row.split_whitespace().next().unwrap().parse().unwrap();
    assert_eq!(at, want, "{row}");
    assert!(row.ends_with(") -> String"), "{row}");
    assert!(
        out.contains("  pub fn parse_routing(reply) -> Routing\n"),
        "{out}"
    );
}

/// Top-level rows all stay when the nested ones do not fit, and the last
/// line counts exactly the nested ones that went.
#[test]
fn a_big_file_keeps_its_top_level_and_counts_what_was_left_out() {
    let mut src = String::new();
    for t in 0..150 {
        src.push_str(&format!("impl Type{t} {{\n"));
        for m in 0..10 {
            src.push_str(&format!(
                "    pub fn method_{m}(&self) -> usize {{\n        {m}\n    }}\n"
            ));
        }
        src.push_str("}\n");
    }
    let out = render("big.rs", &src).unwrap();
    assert!(out.len() <= RUN_FIT, "{} bytes", out.len());
    let rows: Vec<&str> = out.lines().skip(1).collect();
    let (last, rows) = rows.split_last().unwrap();
    let top = rows.iter().filter(|r| r.contains("  impl Type")).count();
    assert_eq!(top, 150, "every impl is listed");
    let nested = rows.len() - top;
    assert!(nested > 0, "nested rows fill what is left");
    assert_eq!(
        *last,
        format!(
            "\u{2026} {} nested definitions left out, every top-level one kept \u{2014} sys:read_file with \"line\" at a block to see inside it",
            grouped(1_500 - nested)
        )
    );
    // What is shown stays in file order.
    let lines: Vec<usize> = rows
        .iter()
        .map(|r| r.split_whitespace().next().unwrap().parse().unwrap())
        .collect();
    assert!(lines.windows(2).all(|w| w[0] < w[1]), "{out}");
}

#[test]
fn a_file_of_too_many_top_level_definitions_says_so_within_budget() {
    let src: String = (0..50_000).map(|i| format!("fn f{i}() {{}}\n")).collect();
    let out = render("gen.rs", &src).unwrap();
    assert!(out.len() <= RUN_FIT, "{} bytes", out.len());
    let kept = out.lines().count() - 2;
    let last = out.lines().last().unwrap();
    assert_eq!(
        last,
        format!(
            "\u{2026} {} definitions left out ({} top-level, 0 nested) \u{2014} too many to list; sys:grep for the name you want",
            grouped(50_000 - kept),
            grouped(50_000 - kept)
        )
    );
    // Five digits of line numbers, so five columns for them.
    assert!(
        out.starts_with("gen.rs \u{2014} 50,000 lines\n    1  fn f0()\n"),
        "{out}"
    );
}

#[test]
fn a_directory_is_sent_to_glob_and_a_missing_file_is_named() {
    let d = dir(&[("lib.rs", "fn a() {}\n")]);
    let e = outline(d.to_str().unwrap()).unwrap_err();
    assert!(e.contains("is a directory"), "{e}");
    assert!(e.contains("sys:glob"), "{e}");
    let missing = d.join("lob.rs");
    let e = outline(missing.to_str().unwrap()).unwrap_err();
    assert!(e.starts_with("outline "), "{e}");
    assert!(e.contains("did you mean \u{201c}lib.rs\u{201d}?"), "{e}");
}

#[test]
fn a_file_with_nothing_to_list_says_so() {
    assert_eq!(
        render("empty.py", "x = 1\n").unwrap(),
        "empty.py \u{2014} 1 line\n  no definitions found \u{2014} read it with sys:read_file"
    );
}

/// Reached through the dispatcher, and offered in the text-mode tool list
/// with the pointer to `sys:read_file`'s `line` inside the list's clip.
#[test]
fn it_is_dispatched_and_its_hint_says_where_to_go_next() {
    let d = dir(&[("lib.rs", "pub fn a() {}\n")]);
    let args = serde_json::json!({"path": d.join("lib.rs")}).to_string();
    assert_eq!(
        systools::call("outline", &args).unwrap(),
        "lib.rs \u{2014} 1 line\n   1  pub fn a()"
    );
    let hint = crate::broker::toolcall::hint_for(&systools::tools());
    let row = hint
        .lines()
        .find(|l| l.starts_with("- sys:outline"))
        .unwrap();
    assert!(
        row.ends_with("then sys:read_file with \"line\": {\"path\": \"src/lib.rs\"}"),
        "{row}"
    );
}
