use super::*;

const DIFF: &str = "\
diff --git a/x.rs b/x.rs
@@ -10,4 +10,5 @@ fn main
 context one
-old alpha
-old beta
+new alpha
+new beta
+new gamma
 context two
";

#[test]
fn a_hunk_header_sets_both_sides_line_numbers() {
    assert_eq!(hunk_start("@@ -10,4 +12,5 @@ fn main"), Some((10, 12)));
    assert_eq!(hunk_start("@@ -1 +1 @@"), Some((1, 1)));
    assert_eq!(hunk_start("not a hunk"), None);
}

/// A context line is ONE line of the file shown twice — same text, same
/// number on both sides — and both counters advance.
#[test]
fn a_context_line_is_the_same_line_on_both_sides() {
    let r = rows(DIFF);
    let Row::Pair(l, lt, n, rt) = &r[2] else {
        panic!("expected a pair, got {:?}", r[2])
    };
    assert_eq!((*l, *n), (Some(10), Some(10)));
    assert_eq!(lt, rt);
}

/// Removals and additions are gathered into runs and paired index by index —
/// the same correspondence the unified rung refines. The overhang of the
/// longer run lands on its own side, alone.
#[test]
fn runs_pair_index_by_index_and_the_overhang_stands_alone() {
    let r = rows(DIFF);
    type Pair<'a> = (
        Option<usize>,
        Option<&'a str>,
        Option<usize>,
        Option<&'a str>,
    );
    let pairs: Vec<Pair> = r
        .iter()
        .filter_map(|row| match row {
            Row::Pair(a, b, c, d) => Some((*a, *b, *c, *d)),
            _ => None,
        })
        .collect();
    assert_eq!(
        pairs[1],
        (Some(11), Some("-old alpha"), Some(11), Some("+new alpha"))
    );
    assert_eq!(
        pairs[2],
        (Some(12), Some("-old beta"), Some(12), Some("+new beta"))
    );
    assert_eq!(
        pairs[3],
        (None, None, Some(13), Some("+new gamma")),
        "the extra addition has nothing on its left"
    );
}

/// The counters keep the FILE's numbering: two lines removed and three added
/// puts the context line after them on 13 in the old file and 14 in the new.
#[test]
fn each_side_counts_its_own_file() {
    let r = rows(DIFF);
    let after = r
        .iter()
        .find_map(|row| match row {
            Row::Pair(l, Some(" context two"), n, _) => Some((*l, *n)),
            _ => None,
        })
        .expect("the context line after the change");
    assert_eq!(after, (Some(13), Some(14)));
}

#[test]
fn headers_span_both_sides() {
    let r = rows(DIFF);
    assert!(matches!(r[0], Row::Full(_, Kind::File)));
    assert!(matches!(r[1], Row::Full(_, Kind::Hunk)));
}

/// A removal with no addition after it (a pure deletion) has an empty right
/// side rather than being paired with whatever came next.
#[test]
fn a_pure_deletion_has_nothing_on_its_right() {
    let r = rows("@@ -1,2 +1,1 @@\n-gone\n keep\n");
    let Row::Pair(l, lt, n, rt) = &r[1] else {
        panic!("expected a pair")
    };
    assert_eq!((*l, *lt), (Some(1), Some("-gone")));
    assert_eq!((*n, *rt), (None, None));
}

#[test]
fn nothing_in_no_rows_out() {
    assert!(rows("")
        .iter()
        .all(|r| matches!(r, Row::Pair(_, Some(""), _, Some("")))));
}
