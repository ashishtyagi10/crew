//! `sys:edit {"line": N}`, through the real tool on a real file. The shape is
//! the 2026-10-22 probe's: two functions with the same `if r >= floor {` line,
//! where qwen-max sent `"line": 84` / `"line": 131` six times each, every
//! one dropped and refused as "appears 2 times".
use crate::broker::systools::call;
use serde_json::json;

const BODY: &str = "\
fn best(r: f64, floor: f64) -> bool {
    if r >= floor {
        return true;
    }
    false
}

fn worst(r: f64, floor: f64) -> bool {
    if r >= floor {
        return true;
    }
    false
}
";

struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("crew-editpick-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("floor.rs");
        std::fs::write(&p, BODY).unwrap();
        Scratch(p)
    }
    fn edit(&self, args: serde_json::Value) -> Result<String, String> {
        let mut args = args;
        args["path"] = json!(self.0);
        call("edit", &args.to_string())
    }
    fn now(&self) -> String {
        std::fs::read_to_string(&self.0).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

#[test]
fn a_repeat_is_refused_with_the_lines_it_is_on() {
    let f = Scratch::new("where");
    let err = f
        .edit(json!({"old": "    if r >= floor {", "new": "    if r > floor {"}))
        .unwrap_err();
    assert!(
        err.contains(": `old` appears 2 times (lines 2 and 9)"),
        "{err}"
    );
    assert!(err.contains("pass \"line\""), "{err}");
    assert_eq!(f.now(), BODY, "untouched");
}

#[test]
fn a_line_picks_the_copy_it_is_in_and_the_report_names_that_line() {
    let f = Scratch::new("pick");
    let got = f
        .edit(json!({"old": "    if r >= floor {", "new": "    if r > floor {", "line": 9}))
        .unwrap();
    assert!(got.starts_with("edited floor.rs at line 9"), "{got}");
    let (head, tail) = BODY.split_at(BODY.find("fn worst").unwrap());
    assert_eq!(
        f.now(),
        format!("{head}{}", tail.replace(">= floor", "> floor"))
    );
}

#[test]
fn a_line_anywhere_inside_a_multi_line_old_names_it_and_a_quoted_one_reads() {
    let f = Scratch::new("inside");
    let old = "    if r >= floor {\n        return true;";
    let new = "    if r > floor {\n        return true;";
    f.edit(json!({"old": old, "new": new, "line": "10"}))
        .unwrap();
    assert!(f
        .now()
        .starts_with("fn best(r: f64, floor: f64) -> bool {\n    if r >= floor"));
    assert!(f
        .now()
        .contains("fn worst(r: f64, floor: f64) -> bool {\n    if r > floor"));
}

#[test]
fn a_line_on_no_copy_is_refused_rather_than_guessed_at() {
    let f = Scratch::new("stale");
    let args = json!({"old": "    if r >= floor {", "new": "x", "line": 5});
    let err = f.edit(args).unwrap_err();
    assert!(err.contains("none of them is on line 5"), "{err}");
    assert!(err.contains("(lines 2 and 9)"), "{err}");
    assert_eq!(f.now(), BODY, "untouched");
}

#[test]
fn two_on_the_line_itself_ask_for_more_of_the_line() {
    let f = Scratch::new("sameline");
    let err = f
        .edit(json!({"old": "r: f64", "new": "r: f32", "line": 1}))
        .unwrap_err();
    // `r: f64` is once per signature line, so this one is unique per line...
    assert!(
        err.contains("none of them") || err.contains("line 1 itself"),
        "{err}"
    );
    let err = f
        .edit(json!({"old": "f64", "new": "f32", "line": 1}))
        .unwrap_err();
    assert!(err.contains("appears 2 times on line 1 itself"), "{err}");
}

#[test]
fn in_a_batch_every_line_means_the_file_as_it_was_read() {
    // The first edit adds two lines above both copies; the second names the
    // copy on line 2 as read, which now sits on line 4. Without the shift it
    // matches nothing; with copies two lines apart it would pick the wrong one.
    let f = Scratch::new("batch");
    let edits = json!([
        {"old": "fn best(", "new": "// a\n// b\nfn best("},
        {"old": "    if r >= floor {", "new": "    if r > floor {", "line": 2},
    ]);
    f.edit(json!({"edits": edits})).unwrap();
    let now = f.now();
    assert!(
        now.starts_with("// a\n// b\nfn best(r: f64, floor: f64) -> bool {\n    if r > floor {"),
        "{now}"
    );
    assert!(
        now.contains("fn worst(r: f64, floor: f64) -> bool {\n    if r >= floor {"),
        "{now}"
    );
}

#[test]
fn a_unique_old_still_lands_whatever_line_rides_with_it() {
    let f = Scratch::new("unique");
    f.edit(json!({"old": "fn worst", "new": "fn least", "line": 40}))
        .unwrap();
    assert!(f.now().contains("fn least("));
}
