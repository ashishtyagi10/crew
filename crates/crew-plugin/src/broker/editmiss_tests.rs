use super::super::editpick::Shift;
use super::super::sysedit::{place, Swap};

/// The refusal `sys:edit` gives `old` → `new` against `body`.
fn miss(body: &str, old: &str, new: &str) -> String {
    let s = Swap {
        old,
        new,
        all: false,
        line: None,
    };
    place(body, &s, &Shift::default()).unwrap_err()
}

/// `crew-theme`'s `contrast_ratio` as the a14 probe planted it, under a line
/// of its own so it starts at line 2.
const RATIO: &str = "/// WCAG 2.1 contrast ratio between two sRGB colours.
pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    ((hi + 0.05) / (lo + 0.05)) as f64
}
";

/// a14x2 sent this `old` five rounds running and was told only that "the
/// rest of `old` does not match what follows it". The answer names the line
/// it parts at, and shows the file's lines from there to copy.
#[test]
fn an_old_that_parts_partway_says_where_and_shows_the_file() {
    let old = "pub fn contrast_ratio(a: (u8, u8, u8), b: (u8, u8, u8)) -> f64 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}";
    let new = old.replace("-> f64", "-> f32");
    let err = miss(RATIO, old, &new);
    assert!(
        err.contains("its first 4 lines are lines 2\u{2013}5"),
        "{err}"
    );
    assert!(
        err.contains("\u{201c}(hi + 0.05) / (lo + 0.05)\u{201d}, line 6 reads"),
        "{err}"
    );
    assert!(
        err.ends_with("6\u{2502}     ((hi + 0.05) / (lo + 0.05)) as f64\n7\u{2502} }"),
        "{err}"
    );
}

/// a14off2 sent two lines 33 apart as one `old`: no single place holds it,
/// and the answer says where each part is.
#[test]
fn an_old_from_two_places_names_each() {
    let body = "            assert!(
                ratio >= want - 0.02,
                \"{} slot {slot}\",
            );
        }
    for id in ALL_THEMES {
            if r < raised - 0.02 {
                thin.push(id);
";
    let err = miss(
        body,
        "ratio >= want - 0.02,\nif r < raised - 0.02 {",
        "ratio >= (want - 0.02) as f64,\nif r < (raised - 0.02) as f64 {",
    );
    assert!(err.contains("in 2 pieces"), "{err}");
    assert!(
        err.contains("its line 1 is line 2; its line 2 is line 7"),
        "{err}"
    );
    assert!(err.contains("\"edits\""), "{err}");
}

/// Two blocks of a test sent as one, a blank line between; `.collect();` is
/// in both, but each block as a whole is in one place.
#[test]
fn two_blocks_sent_as_one_are_two_places() {
    let block = |ty: &str| {
        format!("        let mut v: Vec<{ty}> = ids\n            .iter()\n            .map(|id| ratio(id))\n            .collect();\n")
    };
    let body = format!(
        "fn a() {{\n{}    sort(v);\n}}\n\nfn b() {{\n{}}}\n",
        block("(f32, &str)"),
        block("f32")
    );
    let old = format!("{}\n\n{}", block("(f32, &str)").trim(), block("f32").trim());
    let err = miss(&body, &old, &old.replace("f32", "f64"));
    assert!(
        err.contains("its lines 1\u{2013}4 are lines 2\u{2013}5"),
        "{err}"
    );
    assert!(
        err.contains("its lines 6\u{2013}9 are lines 10\u{2013}13"),
        "{err}"
    );
}

/// a14off2 redid an edit it had made: the old refusal pointed it at the
/// nearest line, which was its own `new`, to copy.
#[test]
fn a_new_already_there_says_so() {
    let body = "        for slot in 0..8 {\n            if r < (raised - 0.02) as f64 {\n";
    let err = miss(
        body,
        "if r < raised - 0.02 {",
        "if r < (raised - 0.02) as f64 {",
    );
    assert!(err.contains("`new` already is (line 2)"), "{err}");
    assert!(
        err.ends_with("2\u{2502}             if r < (raised - 0.02) as f64 {"),
        "{err}"
    );
}

/// A deletion's `new` is in the file whether or not the edit was made, and
/// a short one is in it by chance: neither is called made.
#[test]
fn a_deletion_or_a_short_new_is_not_called_made() {
    let body = "let a = first();\nlet b = 2;\n";
    let err = miss(body, "let a = first();\nlet c = 3;", "let a = first();");
    assert!(!err.contains("already"), "{err}");
    assert!(err.contains("then it parts"), "{err}");
    let err = miss(body, "let z = 9;", "b = 2;");
    assert!(!err.contains("already"), "{err}");
}

/// An `old` that runs past the end of the file says so.
#[test]
fn an_old_past_the_end_says_the_file_ends() {
    let err = miss("fn a() {}\nfn b() {}\n", "fn b() {}\nfn c() {}", "x");
    assert!(
        err.ends_with("where the file ends and `old` has \u{201c}fn c() {}\u{201d}"),
        "{err}"
    );
}

/// In a batch, "nothing was written" ends the reason's first line, not the
/// last row it shows, where it read as part of the file.
#[test]
fn in_a_batch_the_rows_stay_the_files_own() {
    let dir = std::env::temp_dir().join(format!("crew-editmiss-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("lib.rs");
    std::fs::write(&path, RATIO).unwrap();
    let swap = |old, new| Swap {
        old,
        new,
        all: false,
        line: None,
    };
    let old = "let l2 = relative_luminance(b);\n    let hi = l2;";
    let swaps = [swap("-> f64", "-> f32"), swap(old, "x")];
    let err = super::super::sysedit::edit_all(path.to_str().unwrap(), &swaps).unwrap_err();
    let first = err.lines().next().unwrap();
    assert!(first.ends_with("\u{2014} nothing was written"), "{err}");
    assert!(
        err.ends_with("5\u{2502}     let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };"),
        "{err}"
    );
    std::fs::remove_dir_all(dir).ok();
}
