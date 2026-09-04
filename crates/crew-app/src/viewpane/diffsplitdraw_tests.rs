use super::*;

const DIFF: &str = "\
diff --git a/x.rs b/x.rs
@@ -10,3 +10,3 @@ fn main
 context one
-let alpha = 1;
+let alpha = 2;
";

fn text_of(line: &CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}

fn render(cols: usize) -> Vec<String> {
    let _g = crate::app::theme_test_guard();
    lines(DIFF, cols)
        .expect("wide enough")
        .0
        .iter()
        .map(text_of)
        .collect()
}

/// Below two honest columns there is no split, and the caller falls back to
/// the unified rung rather than being handed something unreadable.
#[test]
fn a_narrow_pane_is_told_no() {
    let _g = crate::app::theme_test_guard();
    assert!(lines(DIFF, MIN_COLS - 1).is_none());
    assert!(lines(DIFF, MIN_COLS).is_some());
}

/// Every row is exactly the pane's width — with the divider in the SAME
/// column on every one of them, or the split reads as two lists that happen
/// to be next to each other.
#[test]
fn the_divider_is_a_straight_line_at_every_width() {
    for cols in MIN_COLS..MIN_COLS + 40 {
        let rendered = render(cols);
        let at = (cols - 1) / 2;
        for (i, row) in rendered.iter().enumerate() {
            let chars: Vec<char> = row.chars().collect();
            // Header rows span both columns and carry no divider.
            if chars.get(at) == Some(&DIVIDER) {
                continue;
            }
            assert!(
                i < 2,
                "row {i} at {cols} columns has no divider at {at}: {row:?}"
            );
        }
    }
}

/// The old line is on the left and the new one on the right, on the SAME row
/// — which is the whole point: a removed line and its replacement occupy the
/// same place in the file.
#[test]
fn the_pair_sits_on_one_row() {
    let rendered = render(80);
    let row = rendered
        .iter()
        .find(|r| r.contains("alpha = 1"))
        .expect("the old line is drawn");
    assert!(row.contains("alpha = 2"), "…beside the new one: {row:?}");
    let at = row.find("alpha = 1").unwrap();
    let to = row.find("alpha = 2").unwrap();
    assert!(at < to, "old is on the left");
}

/// Each side carries its own file's numbering, from the hunk header.
#[test]
fn both_sides_number_their_own_file() {
    let rendered = render(80);
    let row = rendered.iter().find(|r| r.contains("context one")).unwrap();
    assert!(row.trim_start().starts_with("10"), "left number: {row:?}");
    let right: String = row.chars().skip((80 - 1) / 2 + 1).collect();
    assert!(right.trim_start().starts_with("10"), "right: {right:?}");
}

/// A long line wraps INSIDE its half and the other side is padded to match,
/// so the two versions never slide out of step — which is exactly where they
/// are long enough to need the help.
#[test]
fn a_wrapped_side_keeps_the_other_in_step() {
    let _g = crate::app::theme_test_guard();
    let long = "x".repeat(200);
    let diff = format!("@@ -1,1 +1,1 @@\n-{long}\n+short\n");
    let (rendered, _) = lines(&diff, MIN_COLS).expect("wide enough");
    let rows: Vec<String> = rendered.iter().map(text_of).collect();
    let wrapped: Vec<&String> = rows.iter().filter(|r| r.contains('x')).collect();
    assert!(
        wrapped.len() > 1,
        "the long side wrapped: {}",
        wrapped.len()
    );
    let at = (MIN_COLS - 1) / 2;
    for row in &rows[1..] {
        let chars: Vec<char> = row.chars().collect();
        assert_eq!(chars.get(at), Some(&DIVIDER), "{row:?} lost its divider");
        assert_eq!(chars.len(), MIN_COLS, "{row:?} is not the pane's width");
    }
    // The short side is drawn once and blank underneath — never repeated.
    assert_eq!(rows.iter().filter(|r| r.contains("short")).count(), 1);
}

/// A side with nothing on it is blank, not a copy of its partner: this is
/// where one version of the file simply has no line, and drawing anything
/// would invent one.
#[test]
fn a_side_with_no_line_is_blank() {
    let _g = crate::app::theme_test_guard();
    let (rendered, _) = lines("@@ -1,1 +1,0 @@\n-gone\n", 80).expect("wide enough");
    let row = rendered
        .iter()
        .map(text_of)
        .find(|r| r.contains("gone"))
        .expect("the deletion is drawn");
    let right: String = row.chars().skip((80 - 1) / 2 + 1).collect();
    assert_eq!(
        right.trim(),
        "",
        "the right side invented a line: {right:?}"
    );
}

/// The word-level refinement comes with: the shared text of a pair recedes
/// and only the run that changed is at full strength, on both sides.
#[test]
fn the_refinement_comes_with_the_split() {
    let _g = crate::app::theme_test_guard();
    let (rendered, _) = lines(DIFF, 80).expect("wide enough");
    let row = rendered
        .iter()
        .find(|r| text_of(r).contains("alpha = 1"))
        .expect("the pair");
    let inks: Vec<(u8, u8, u8)> = row
        .iter()
        .filter(|c| c.c == 'a' || c.c == '1' || c.c == '2')
        .map(|c| c.fg)
        .collect();
    assert!(
        inks.iter().collect::<std::collections::HashSet<_>>().len() > 1,
        "everything is one colour — nothing was refined"
    );
}

/// A header wider than the pane is cut and SAYS so; it used to stop
/// mid-path in silence while the body rows beside it fitted.
#[test]
fn a_wide_header_marks_its_cut() {
    let _g = crate::app::theme_test_guard();
    let diff = "diff --git a/some/deeply/nested/directory/file.rs b/some/deeply/nested/directory/file.rs\n@@ -1,1 +1,1 @@\n-a\n+b\n";
    let rows = lines(diff, MIN_COLS).expect("wide enough").0;
    let head = text_of(&rows[0]);
    assert_eq!(head.chars().count(), MIN_COLS, "{head:?}");
    assert!(head.ends_with('\u{2026}'), "{head:?}");
}
