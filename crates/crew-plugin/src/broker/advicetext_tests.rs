//! What the advisor is shown, and what of its reply reaches the pane.
use super::*;

fn run(cmd: &str) -> ToolCall {
    ToolCall {
        server: "sys".into(),
        tool: "run".into(),
        args: format!("{{\"cmd\": \"{cmd}\"}}"),
    }
}

#[test]
fn smith_says_it_quietly_with_the_first_line_clipped() {
    assert_eq!(said("read the error"), "advised \u{b7} read the error");
    let long = "word ".repeat(40);
    let line = said(&long);
    assert!(line.starts_with("advised \u{b7} word") && line.ends_with('\u{2026}'));
}

/// A run cut for length (`runfit`) opens with every error by its place
/// (`rundigest`), and the cut to the failure keeps that line: the errors the
/// tail dropped are the ones the advisor most needs to see.
#[test]
fn a_cut_run_s_error_list_reaches_the_brief() {
    let noise: String = (0..400)
        .map(|i| format!("   Compiling c{i} v1.0.0\n"))
        .collect();
    let errors: String = (0..30)
        .map(|i| format!("error[E0308]: mismatched\n  --> src/f{}.rs:{i}:1\n", i % 3))
        .collect();
    let out = crate::broker::runfit::fit(101, "", &format!("{errors}{noise}"), false);
    assert!(!out.contains("src/f0.rs:3:1"), "the cut dropped that error");
    let b = brief("make it build", &[], &run("cargo build"), &out);
    assert!(b.contains("30 errors at src/f0.rs:0,3,6"), "{b}");
}
