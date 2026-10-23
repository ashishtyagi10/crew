use super::digest;

/// The shape of the probe's `cargo test`: errors in three files, a note's
/// arrow under one of them, and the closing `could not compile`.
const CARGO: &str = "   Compiling crew-theme v0.27.21
error[E0308]: mismatched types
  --> crates/crew-theme/src/readable_tests.rs:25:20
   |
25 |         if r < floor {
error[E0277]: a value of type `Vec<(f32, &str)>` cannot be built
   --> crates/crew-theme/src/signal_tests.rs:185:18
note: the method call chain might not have had the expected associated types
   --> crates/crew-theme/src/signal_tests.rs:179:18
warning: unused import
  --> crates/crew-theme/src/lib.rs:3:5
error[E0308]: mismatched types
   --> crates/crew-theme/src/readable_tests.rs:327:20
error[E0308]: mismatched types
   --> crates/crew-theme/src/oklch_tests.rs:133:21
error: could not compile `crew-theme` (lib test) due to 4 previous errors
";

#[test]
fn every_error_is_named_by_its_place_and_nothing_else_is() {
    assert_eq!(
        digest("", CARGO),
        "4 errors at crates/crew-theme/src/readable_tests.rs:25,327 \u{b7} \
         crates/crew-theme/src/signal_tests.rs:185 \u{b7} crates/crew-theme/src/oklch_tests.rs:133\n\
         crates/crew-theme/src/readable_tests.rs\n  25\u{2502} if r < floor { \u{2014} mismatched types\n"
    );
}

/// One error is in the tail anyway, and a run with none says nothing.
#[test]
fn fewer_than_two_say_nothing() {
    assert_eq!(digest("", "error[E0308]: x\n  --> a.rs:1:1\n"), "");
    assert_eq!(digest("test result: ok", ""), "");
}

/// Past its room the line says how many files it left out.
#[test]
fn a_long_list_says_what_it_left_out() {
    let many: String = (0..40)
        .map(|i| format!("error[E0308]: x\n  --> crates/crew-theme/src/file_number_{i}.rs:{i}:1\n"))
        .collect();
    let got = digest("", &many);
    assert!(got.len() < 500, "{}", got.len());
    assert!(got.starts_with("40 errors at"), "{got}");
    assert!(got.trim_end().ends_with("more files"), "{got}");
}

/// Through the fitting: a run cut for length names its errors under its
/// exit line, where the cut part's errors would otherwise be gone.
#[test]
fn a_cut_run_names_its_errors_under_the_exit_line() {
    let noise: String = (0..400)
        .map(|i| format!("   Compiling crate-{i} v1.0.0\n"))
        .collect();
    let errors: String = (0..30)
        .map(|i| {
            format!(
                "error[E0308]: mismatched types\n  --> src/f{}.rs:{i}:1\n   |\n",
                i % 3
            )
        })
        .collect();
    let out = crate::broker::runfit::fit(101, "", &format!("{errors}{noise}"), false);
    let second = out.lines().nth(1).unwrap_or_default();
    assert!(
        second.starts_with("30 errors at src/f0.rs:0,3,6"),
        "{second}"
    );
    assert!(out.contains("src/f2.rs:2,5"), "the cut ones are named too");
}

/// The compile task's real `cargo test` (25 errors, 17.9 KB): every error
/// rustc quoted a line for comes back as that line, copyable as `old`, with
/// what rustc said of it; a note's or help's code is not quoted.
#[test]
fn a_real_build_quotes_each_errors_line() {
    let got = digest("", include_str!("testdata/cargo_f64_errors.txt"));
    let rows: Vec<&str> = got.lines().skip(1).collect();
    assert!(got.starts_with("25 errors at"), "{got}");
    assert!(
        got.contains("crates/crew-theme/src/oklch.rs\n  220\u{2502} if contrast_ratio(at(far), bg) < target { \u{2014} expected `f64`, found `f32`\n"),
        "{got}"
    );
    assert!(
        rows.contains(&"  90\u{2502} .fold(f32::INFINITY, f32::min) \u{2014} type mismatch in function arguments"),
        "{got}"
    );
    assert!(
        !got.contains("iterator.rs") && !got.contains("target.into()"),
        "{got}"
    );
    assert_eq!(
        rows.iter().filter(|r| r.contains('\u{2502}')).count(),
        25,
        "all 25 quoted: {got}"
    );
    assert!(got.len() <= 3_100, "{}", got.len());
}
