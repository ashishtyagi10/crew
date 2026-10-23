use super::respace;

/// `signal_tests.rs`'s band check, as the a10x2 probe met it: the chain sits
/// 16 spaces in, under a `.map(|(_, ids)| {` at 12.
const BAND: &str = "            .map(|(_, ids)| {
                let mut v: Vec<f32> = ids
                    .iter()
                    .map(|id| contrast_ratio(get(id.theme()), id.theme().page_bg))
                    .collect();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap());
";

/// a10x2 sent the chain 12 spaces short, and the edit came back "the same
/// text is there with different whitespace" twice. It lands now, as the
/// file has it, and `new` is moved in by the same 12.
#[test]
fn a_block_off_by_one_indent_lands_as_the_file_has_it() {
    let old = "    let mut v: Vec<f32> = ids\n        .iter()\n        .map(|id| contrast_ratio(get(id.theme()), id.theme().page_bg))\n        .collect();";
    let new = old.replace("f32", "f64");
    let (theirs, moved) = respace(BAND, old, &new).expect("lands");
    assert!(BAND.contains(&theirs), "{theirs}");
    assert!(theirs.starts_with("                let mut v"), "{theirs}");
    assert!(
        moved.starts_with("                let mut v: Vec<f64> = ids\n                    .iter()"),
        "{moved}"
    );
    assert!(
        moved.ends_with("\n                    .collect();"),
        "{moved}"
    );
}

/// The other way, 4 spaces too many (a1base1), and with no indent at all.
#[test]
fn too_much_or_none_lands_too() {
    let body = "fn a() {\n    let x = 1;\n    let y = 2;\n}\n";
    let (theirs, moved) = respace(
        body,
        "        let x = 1;\n        let y = 2;",
        "        let x = 3;\n        let y = 4;",
    )
    .unwrap();
    assert_eq!(theirs, "    let x = 1;\n    let y = 2;");
    assert_eq!(moved, "    let x = 3;\n    let y = 4;");
    let (_, moved) = respace(
        body,
        "let x = 1;\nlet y = 2;\n",
        "let x = 3;\n\nlet y = 4;\n",
    )
    .unwrap();
    assert_eq!(
        moved, "    let x = 3;\n\n    let y = 4;\n",
        "a blank line stays blank"
    );
}

/// Anything looser than one shift, one place, stays a refusal.
#[test]
fn uneven_twice_or_one_line_is_still_a_miss() {
    let body = "fn a() {\n    let x = 1;\n    let y = 2;\n}\nfn b() {\n    let x = 1;\n    let y = 2;\n}\n";
    assert_eq!(
        respace(body, "let x = 1;\nlet y = 2;", "x"),
        None,
        "two places"
    );
    let body = "fn a() {\n    let x = 1;\n        let y = 2;\n}\n";
    assert_eq!(
        respace(body, "let x = 1;\nlet y = 2;", "x"),
        None,
        "two shifts"
    );
    assert_eq!(respace(body, "  let x = 1;", "x"), None, "one line");
    let body = "    let x = 1;\n    let y = 2;\n";
    assert_eq!(
        respace(
            body,
            "        let x = 1;\n        let y = 2;",
            "  let x = 3;\n  let y = 4;"
        ),
        None,
        "new has less than the 4 to take off"
    );
}

/// Through `sys:edit`: the file is edited where `old` meant, and the report
/// says it was matched by the file's indentation, not as sent.
#[test]
fn the_edit_says_it_matched_by_the_files_indentation() {
    let dir = std::env::temp_dir().join(format!("crew-editindent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("signal_tests.rs");
    std::fs::write(&path, BAND).unwrap();
    let old = "let mut v: Vec<f32> = ids\n    .iter()";
    let swap = super::super::sysedit::Swap {
        old,
        new: "let mut v: Vec<f64> = ids\n    .iter()",
        all: false,
        line: None,
    };
    let said = super::super::sysedit::edit_all(path.to_str().unwrap(), &[swap]).unwrap();
    assert!(
        said.starts_with("edited signal_tests.rs at line 2 (2 lines) \u{2014} matched with the file's indentation"),
        "{said}"
    );
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(after, BAND.replace("Vec<f32>", "Vec<f64>"));
    std::fs::remove_dir_all(dir).ok();
}
