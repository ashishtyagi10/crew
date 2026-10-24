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

/// A run cut for length (crew-plugin `runfit`) opens with every error by its
/// place (`rundigest`), and the cut to the failure keeps that line: the
/// errors the tail dropped are the ones the advisor most needs to see.
#[test]
fn a_cut_run_s_error_list_reaches_the_brief() {
    let noise: String = (0..400)
        .map(|i| format!("   Compiling c{i} v1.0.0\n"))
        .collect();
    let out = format!(
        "exit 101\n30 errors at src/f0.rs:0,3,6,9 \u{b7} src/f1.rs:1,4,7\n\u{2026}\n{noise}\
         error[E0308]: mismatched\n  --> src/f2.rs:29:1\nerror: could not compile"
    );
    let b = brief("make it build", &[], &run("cargo build"), &out);
    assert!(b.contains("30 errors at src/f0.rs:0,3,6"), "{b}");
    assert!(
        b.contains("src/f2.rs:29:1") && !b.contains("Compiling c200"),
        "{b}"
    );
}
