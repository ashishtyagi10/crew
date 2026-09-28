use super::*;
use crate::broker::toolclip::{clip_result, AGENT_CLIP};

/// Real crate names, so the progress reads as a real build's does.
const CRATES: &[&str] = &[
    "proc-macro2 v1.0.86",
    "unicode-ident v1.0.12",
    "serde v1.0.210",
    "libc v0.2.158",
    "cfg-if v1.0.0",
    "memchr v2.7.4",
    "quote v1.0.37",
    "syn v2.0.77",
];

/// What cargo prints on stderr before it has anything to say: 80 `Compiling`
/// lines and a dozen warnings.
fn progress() -> String {
    let mut s = String::new();
    for i in 0..80 {
        s.push_str(&format!("   Compiling {}\n", CRATES[i % CRATES.len()]));
    }
    for i in 0..12 {
        s.push_str(&format!(
            "warning: unused variable: `stream{i}`\n   --> crates/crew-plugin/src/broker/toolcall.rs:{}:9\n    |\n{0:>3} |         stream{i}: &HopStream,\n    |         ^^^^^^^ help: if this is intentional, prefix it with an underscore: `_stream{i}`\n    |\n    = note: `#[warn(unused_variables)]` on by default\n\n",
            100 + i
        ));
    }
    s
}

/// `cargo test` on a workspace that no longer compiles: the error comes last.
fn broken_build_stderr() -> String {
    let mut s = progress();
    s.push_str(
        "error[E0308]: mismatched types\n   --> crates/crew-plugin/src/broker/sysrun.rs:140:5\n    |\n140 |     Ok(text.len())\n    |     ^^^^^^^^^^^^^^ expected `String`, found `usize`\n\nFor more information about this error, try `rustc --explain E0308`.\nwarning: `crew-plugin` (lib test) generated 12 warnings\nerror: could not compile `crew-plugin` (lib test) due to 1 previous error; 12 warnings emitted\n",
    );
    s
}

/// What an agent reads, through the relay's clip; the swarm's keeps only a
/// head, so passing unchanged here and staying under the clip covers both.
fn shown(text: &str) -> String {
    clip_result(text, AGENT_CLIP)
}

#[test]
fn a_short_result_comes_back_byte_identical() {
    assert_eq!(fit(0, "hi\n", "", false), "exit 0\nhi\n");
    let both = fit(3, "x\n", "oops\n", false);
    assert_eq!(both, "exit 3\nx\n\n--- stderr ---\noops\n");
    assert_eq!(both, whole(3, "x\n", "oops\n", false));
    assert!(!both.contains("cut"), "{both}");
}

#[test]
fn a_broken_build_keeps_its_error_through_the_agent_clip() {
    let stderr = broken_build_stderr();
    assert!(
        whole(101, "", &stderr, false).len() > RUN_FIT,
        "the fixture must need fitting"
    );
    let out = fit(101, "", &stderr, false);
    assert!(out.starts_with("exit 101\n"), "{out}");
    assert!(out.contains("error[E0308]: mismatched types"), "{out}");
    assert!(out.contains("expected `String`, found `usize`"), "{out}");
    assert!(out.ends_with("12 warnings emitted\n"), "{out}");
    assert!(
        out.contains("lines of output cut \u{2014} the end is below)"),
        "{out}"
    );
    assert_eq!(shown(&out), out, "the relay's clip touched a fitted result");
    assert!(out.chars().count() <= RUN_FIT);
}

/// `cargo test` with one failing test: compile progress on stderr, the
/// verdict on stdout, both at the end of their stream.
#[test]
fn a_failed_test_run_keeps_the_verdict_on_stdout_and_the_error_on_stderr() {
    let mut stdout = String::from("\nrunning 600 tests\n");
    for i in 0..600 {
        stdout.push_str(&format!("test broker::case_{i} ... ok\n"));
    }
    stdout.push_str(
        "\nfailures:\n\n---- broker::runfit::tests::fits stdout ----\n\nthread 'broker::runfit::tests::fits' panicked at crates/crew-plugin/src/broker/runfit_tests.rs:9:5:\nassertion `left == right` failed\n  left: 1\n right: 2\n\n\nfailures:\n    broker::runfit::tests::fits\n\ntest result: FAILED. 599 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.20s\n\n",
    );
    let mut stderr = progress();
    stderr.push_str("    Finished `test` profile [unoptimized + debuginfo] target(s) in 41.02s\n     Running unittests src/lib.rs (target/debug/deps/crew_plugin-5f0c)\nerror: test failed, to rerun pass `-p crew-plugin --lib`\n");
    let out = fit(101, &stdout, &stderr, false);
    assert!(out.starts_with("exit 101\n\nrunning 600 tests\n"), "{out}");
    assert!(
        out.contains("test result: FAILED. 599 passed; 1 failed"),
        "{out}"
    );
    assert!(out.contains("panicked at crates/crew-plugin"), "{out}");
    assert!(out.contains("error: test failed, to rerun pass"), "{out}");
    assert_eq!(shown(&out), out);
}

/// Head, gap and tail account for every line: the count is not decoration.
#[test]
fn the_gap_line_counts_exactly_the_lines_it_replaced() {
    let stdout: String = (0..1_000).map(|i| format!("line {i}\n")).collect();
    let out = fit(0, &stdout, "", false);
    let gap = out
        .lines()
        .find(|l| l.contains("lines of output cut"))
        .expect("a gap line");
    let n: usize = gap
        .trim_start_matches("\u{2026} (")
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let kept = out.lines().filter(|l| l.starts_with("line ")).count();
    assert_eq!(kept + n, 1_000, "{out}");
    assert!(
        out.contains("line 0\n") && out.ends_with("line 999\n"),
        "{out}"
    );
}

#[test]
fn one_enormous_line_is_cut_on_a_char_boundary_not_dropped() {
    let giant = "\u{e9}".repeat(40_000); // 80 KB, no newline, two bytes a char
    for (stdout, stderr) in [(giant.as_str(), ""), ("", giant.as_str())] {
        let out = fit(1, stdout, stderr, false);
        assert!(out.starts_with("exit 1\n"), "{out}");
        assert!(out.len() <= RUN_FIT, "{}", out.len());
        assert!(out.ends_with('\u{e9}'), "the end went missing");
    }
}

#[test]
fn the_64_kb_note_survives_the_fitting() {
    let stdout = "x\n".repeat(40_000);
    let out = fit(0, &stdout, "", true);
    assert!(
        out.ends_with("\u{2026} (output truncated at 64 KB)"),
        "{out}"
    );
    assert_eq!(shown(&out), out);
}

#[test]
fn failed_reads_the_exit_line_and_nothing_else() {
    assert!(failed("exit 101\n   Compiling x v0.1.0\n"));
    assert!(failed("exit -1\n"), "killed by a signal");
    assert!(failed("exit 3"));
    assert!(!failed(
        "exit 0\nerror: this line is output, not a verdict\n"
    ));
    assert!(!failed("(empty result)"));
    assert!(!failed("exit strategy: none\n"), "an MCP tool's prose");
    assert!(!failed(""));
}
