use super::*;

/// Two commits' worth of `--line-porcelain`: the first line of each run
/// carries the headers, later lines repeat only the sha.
const PORCELAIN: &str = "\
a1b2c3d4e5f6 1 1 2
author Ada Lovelace
author-mail <ada@example.com>
summary first
\tfn main() {
a1b2c3d4e5f6 2 2
\t    println!(\"hi\");
9f8e7d6c5b4a 3 3 1
author Grace Hopper
summary second
\t}
";

#[test]
fn each_line_gets_its_commit_and_the_author_is_remembered_for_the_run() {
    let lines = parse(PORCELAIN);
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].sha, "a1b2c3d");
    assert_eq!(lines[0].author, "Ada");
    // The second line repeats the sha with no header block of its own — the
    // author has to come from what the first line said.
    assert_eq!(lines[1].sha, "a1b2c3d");
    assert_eq!(lines[1].author, "Ada", "the run's author is remembered");
    assert_eq!(lines[2].sha, "9f8e7d6");
    assert_eq!(lines[2].author, "Grace");
}

/// Git's own marker for a line that is not committed yet is an all-zero sha,
/// which as seven hex digits would say nothing at all.
#[test]
fn an_uncommitted_line_says_so_instead_of_showing_zeros() {
    let lines = parse(
        "0000000000000000000000000000000000000000 1 1 1\nauthor Not Committed Yet\n\tnew line\n",
    );
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].author, "uncommitted");
    assert!(
        !lines[0].sha.contains('0'),
        "a sha of zeros is not a sha: {:?}",
        lines[0].sha
    );
}

#[test]
fn nothing_in_nothing_out() {
    assert!(parse("").is_empty());
    assert!(parse("not porcelain at all\n").is_empty());
}

fn line(sha: &str, author: &str) -> Line {
    Line {
        sha: sha.into(),
        author: author.into(),
    }
}

/// The point of the column: it marks where one commit's work ends and the
/// next begins, so a run is labelled once and its continuation is blank.
#[test]
fn a_run_is_labelled_once_and_then_left_blank() {
    let lines = vec![
        line("a1b2c3d", "Ada"),
        line("a1b2c3d", "Ada"),
        line("9f8e7d6", "Grace"),
        line("a1b2c3d", "Ada"),
    ];
    let out = labels(&lines, WIDE);
    assert!(out[0].starts_with("a1b2c3d Ada"));
    assert_eq!(out[1].trim(), "", "the run's second line is blank");
    assert!(out[2].starts_with("9f8e7d6 Grace"), "a new commit speaks");
    assert!(
        out[3].starts_with("a1b2c3d Ada"),
        "and the same commit again after a break is a new run"
    );
}

/// Every label is exactly the column width, whatever it says. `parse` only
/// ever produces seven-character shas, so the clip is defence against a
/// FUTURE caller rather than the current one — which is exactly why it is
/// tested with a sha longer than the whole column: without the clip, the
/// padding widens instead of truncating and every row below steps right.
#[test]
fn every_label_is_exactly_the_column_wide() {
    let lines = vec![
        line("a1b2c3d", "Ada"),
        line("9f8e7d6", "Bartholomew-Wentworth"),
        line("0e1d2c3", "uncommitted"),
        line("a-sha-longer-than-the-entire-column", "Overflow"),
    ];
    for width in [NARROW, WIDE] {
        for (i, s) in labels(&lines, width).iter().enumerate() {
            assert_eq!(s.chars().count(), width, "row {i} at width {width}: {s:?}");
        }
    }
}

#[test]
fn a_narrow_column_is_the_sha_alone() {
    let out = labels(&[line("a1b2c3d", "Ada")], NARROW);
    assert_eq!(out[0], "a1b2c3d");
}

#[test]
fn below_the_narrow_column_there_is_no_honest_label() {
    assert!(labels(&[line("a1b2c3d", "Ada")], NARROW - 1).is_empty());
}

/// The column never takes more than a third of the pane: one that crowds out
/// the code it annotates has answered the wrong question.
#[test]
fn the_column_is_never_more_than_a_third_of_the_pane() {
    for cols in 0..200usize {
        match width_for(cols) {
            None => assert!(cols / 3 < NARROW, "{cols} columns could have afforded one"),
            Some(w) => {
                assert!(w == WIDE || w == NARROW);
                assert!(w * 3 <= cols, "{w} columns of {cols} is more than a third");
            }
        }
    }
    assert_eq!(width_for(WIDE * 3), Some(WIDE));
    assert_eq!(width_for(WIDE * 3 - 1), Some(NARROW));
}

/// A gutter that cuts "Ashish Tyagi" to "Ashish T" has invented a person.
/// Every clipped name in crew says it is clipped.
#[test]
fn a_long_author_is_marked_as_cut_not_silently_renamed() {
    let lines = vec![Line {
        sha: "3f2a1b0".into(),
        author: "Ashish Tyagi".into(),
    }];
    let label = labels(&lines, WIDE).remove(0);
    assert!(
        label.contains('\u{2026}'),
        "the name was cut without saying so: {label:?}"
    );
    assert!(label.starts_with("3f2a1b0 "), "{label:?}");
    // Still exactly the gutter's width, so the text column cannot shift.
    assert_eq!(label.chars().count(), WIDE, "{label:?}");
}

/// A name that fits is untouched — the mark has to mean something.
#[test]
fn a_short_author_keeps_every_letter() {
    let lines = vec![Line {
        sha: "a10ff32".into(),
        author: "claude".into(),
    }];
    let label = labels(&lines, WIDE).remove(0);
    assert!(label.starts_with("a10ff32 claude"), "{label:?}");
    assert!(!label.contains('\u{2026}'), "{label:?}");
}
