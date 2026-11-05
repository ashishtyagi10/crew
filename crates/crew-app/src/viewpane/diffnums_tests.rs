use super::numbers;

const PATCH: &str = "\
diff --git a/x.rs b/x.rs
index 1111111..2222222 100644
--- a/x.rs
+++ b/x.rs
@@ -18,3 +18,4 @@ fn main() {
     let a = 1;
-    let b = 2;
+    let b = 3;
+    let c = 4;
     let d = 5;
";

/// The gutter says where in the SOURCE you are. Numbering the patch's own
/// lines made `diff --git` line 1 and the first real code line 6 — the line
/// numbers of a file nobody has open.
#[test]
fn a_diff_is_numbered_by_the_file_not_by_the_patch() {
    let got = numbers(PATCH);
    // The four headers and the hunk header belong to no file.
    assert_eq!(&got[..5], &[None, None, None, None, None]);
    // Context takes the new file's number; the removal takes the old file's,
    // the two additions the new file's, and they do not share one.
    assert_eq!(
        &got[5..10],
        &[Some(18), Some(19), Some(19), Some(20), Some(21)]
    );
}

/// A second hunk restarts from its own header rather than counting on from
/// the first — the arithmetic the `@@` line exists to state.
#[test]
fn a_second_hunk_restarts_at_its_own_header() {
    let two = format!("{PATCH}@@ -90,2 +91,2 @@\n     let e = 6;\n");
    let got = numbers(&two);
    assert_eq!(got[10], None, "the second hunk header");
    assert_eq!(got[11], Some(91));
}

/// Git ends a patch with a newline, so `split` leaves an empty string after
/// the last line. Numbered as context it put a phantom line one past the
/// hunk's end under every `/diff` and `/review`.
#[test]
fn the_last_number_is_the_hunks_last_line() {
    let got = numbers("@@ -1,2 +1,2 @@\n a\n b\n");
    assert_eq!(got, vec![None, Some(1), Some(2), None]);
}

/// `\ No newline at end of file` is git's note, not a line of either file:
/// counted as context it numbered itself and pushed the addition after it
/// one line too far.
#[test]
fn the_no_newline_note_is_not_numbered() {
    let got = numbers("@@ -3,1 +3,1 @@\n-a\n\\ No newline at end of file\n+b\n");
    assert_eq!(got, vec![None, Some(3), None, Some(3), None]);
}

/// An empty line INSIDE a hunk (a context line whose leading space an editor
/// stripped) is still a line, and still numbered.
#[test]
fn an_empty_line_mid_hunk_is_still_numbered() {
    let got = numbers("@@ -1,3 +1,3 @@\n a\n\n b\n");
    assert_eq!(&got[1..4], &[Some(1), Some(2), Some(3)]);
}
