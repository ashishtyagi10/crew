use crate::broker::sysedit::{edit_all, Swap};

/// `line 1` to `line n`, each ended by `eol`.
fn lines(n: usize, eol: &str) -> String {
    (1..=n).map(|i| format!("line {i}{eol}")).collect()
}

/// A scratch `main.rs` holding `body`; its directory goes when this does.
struct Scratch(String);

impl Scratch {
    fn new(tag: &str, body: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("crew-editshow-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("main.rs"), body).unwrap();
        Scratch(dir.join("main.rs").display().to_string())
    }

    /// The whole `sys:edit` result, through the real edit on the real file.
    fn edit(&self, pairs: &[(&str, &str)]) -> String {
        let swaps: Vec<Swap> = pairs.iter().map(|(old, new)| Swap { old, new }).collect();
        edit_all(&self.0, &swaps).unwrap()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let dir = std::path::Path::new(&self.0).parent().unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Just the shown part: everything under the report line.
fn shown(tag: &str, body: &str, pairs: &[(&str, &str)]) -> String {
    let out = Scratch::new(tag, body).edit(pairs);
    out.split_once('\n').unwrap().1.to_string()
}

#[test]
fn an_edit_on_line_41_shows_lines_39_to_43_under_the_report() {
    let out = Scratch::new("a", &lines(100, "\n")).edit(&[("line 41\n", "LINE 41\n")]);
    assert_eq!(
        out,
        "edited main.rs at line 41 (1 line)\n\
         39\u{2502} line 39\n\
         40\u{2502} line 40\n\
         41\u{2502} LINE 41\n\
         42\u{2502} line 42\n\
         43\u{2502} line 43"
    );
}

#[test]
fn one_line_become_three_at_line_10_shows_8_to_14_right_aligned() {
    let out = Scratch::new("b", &lines(100, "\n")).edit(&[("line 10\n", "ten a\nten b\nten c\n")]);
    assert_eq!(
        out,
        "edited main.rs at line 10 (1 line \u{2192} 3, +2)\n \
         8\u{2502} line 8\n \
         9\u{2502} line 9\n\
         10\u{2502} ten a\n\
         11\u{2502} ten b\n\
         12\u{2502} ten c\n\
         13\u{2502} line 11\n\
         14\u{2502} line 12"
    );
}

#[test]
fn edits_far_apart_are_two_regions_in_file_order() {
    // Given bottom first, shown top first: the file's order, not the call's.
    let got = shown(
        "far",
        &lines(100, "\n"),
        &[("line 60\n", "LINE 60\n"), ("line 10\n", "LINE 10\n")],
    );
    let want = [
        " 8\u{2502} line 8",
        " 9\u{2502} line 9",
        "10\u{2502} LINE 10",
        "11\u{2502} line 11",
        "12\u{2502} line 12",
        " \u{22ee}",
        "58\u{2502} line 58",
        "59\u{2502} line 59",
        "60\u{2502} LINE 60",
        "61\u{2502} line 61",
        "62\u{2502} line 62",
    ];
    assert_eq!(got, want.join("\n"));
}

#[test]
fn edits_three_lines_apart_are_one_region() {
    let got = shown(
        "near",
        &lines(100, "\n"),
        &[("line 20\n", "LINE 20\n"), ("line 23\n", "LINE 23\n")],
    );
    let want: Vec<String> = (18..=25)
        .map(|n| match n {
            20 | 23 => format!("{n}\u{2502} LINE {n}"),
            _ => format!("{n}\u{2502} line {n}"),
        })
        .collect();
    assert_eq!(got, want.join("\n"));
}

#[test]
fn a_later_edit_above_moves_an_earlier_one_down() {
    // The first edit landed on line 50; the second adds two lines above it,
    // so it is shown where it now is, 52, not where it was made.
    let got = shown(
        "moved",
        &lines(60, "\n"),
        &[("line 50\n", "L50\n"), ("line 10\n", "a\nb\nc\n")],
    );
    assert!(got.contains("\n52\u{2502} L50\n"), "{got}");
    assert!(got.contains("\n12\u{2502} c\n"), "{got}");
}

#[test]
fn a_later_edit_inside_an_earlier_one_is_one_region() {
    let got = shown(
        "inside",
        &lines(30, "\n"),
        &[("line 10\n", "ten\n"), ("ten\n", "TEN\nten b\n")],
    );
    let want = " 8\u{2502} line 8\n 9\u{2502} line 9\n10\u{2502} TEN\n11\u{2502} ten b\n";
    assert!(got.starts_with(want), "{got}");
    assert!(!got.contains('\u{22ee}'), "split in two: {got}");
}

#[test]
fn whole_lines_taken_out_are_marked_where_they_were() {
    // Without the marker the four lines around a deletion read as four lines
    // that were always together, and "did I take out the right two" is the
    // question the agent is asking.
    let out = Scratch::new("del", &lines(10, "\n")).edit(&[("line 5\nline 6\n", "")]);
    assert_eq!(
        out,
        "edited main.rs at line 5 (2 lines \u{2192} 0, -2)\n\
         3\u{2502} line 3\n\
         4\u{2502} line 4\n \
         \u{2502} \u{2504} 2 lines removed\n\
         5\u{2502} line 7\n\
         6\u{2502} line 8"
    );
    let top = shown("deltop", &lines(10, "\n"), &[("line 1\n", "")]);
    assert_eq!(
        top,
        " \u{2502} \u{2504} 1 line removed\n1\u{2502} line 2\n2\u{2502} line 3"
    );
    let end = shown("delend", &lines(10, "\n"), &[("line 10\n", "")]);
    assert_eq!(
        end,
        "8\u{2502} line 8\n9\u{2502} line 9\n \u{2502} \u{2504} 1 line removed"
    );
}

#[test]
fn part_of_a_line_taken_out_shows_the_line_it_left() {
    // Not whole lines, so no marker: the line itself is what changed.
    let body = "fn f() {\n    let mut x = 1;\n    x\n}\n";
    let got = shown("delpart", body, &[("mut ", "")]);
    assert_eq!(
        got,
        "1\u{2502} fn f() {\n2\u{2502}     let x = 1;\n3\u{2502}     x\n4\u{2502} }"
    );
    // Taking out a newline joins two lines, and the joined line is shown.
    let got = shown("join", body, &[("1;\n", "1; ")]);
    assert!(
        got.contains("2\u{2502}     let mut x = 1;     x\n"),
        "{got}"
    );
}

#[test]
fn a_crlf_file_numbers_the_same_and_shows_no_carriage_returns() {
    let s = Scratch::new("crlf", &lines(100, "\r\n"));
    let out = s.edit(&[("line 41\r\n", "LINE 41\r\n")]);
    assert!(!out.contains('\r'), "{out:?}");
    let block = out.split_once('\n').unwrap().1;
    assert_eq!(
        block,
        "39\u{2502} line 39\n40\u{2502} line 40\n41\u{2502} LINE 41\n42\u{2502} line 42\n43\u{2502} line 43"
    );
    // The number is the one grep reports, and the one read_file starts at.
    let hits =
        crate::broker::sysgrep::grep(&serde_json::json!({"pattern": "LINE 41", "path": s.0}))
            .unwrap();
    assert!(hits.contains(":41: LINE 41"), "{hits}");
    let page = crate::broker::sysreadline::read(&s.0, &serde_json::json!({"line": 41})).unwrap();
    assert!(page.starts_with(" 41\u{2502} LINE 41\r\n"), "{page:?}");
}
