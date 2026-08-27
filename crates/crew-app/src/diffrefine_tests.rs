use super::*;
use crate::chatbody::plain;

fn line(text: &str, tok: Option<Token>) -> CardLine {
    let fg = match tok {
        Some(t) => crate::chatink::token_fg(t),
        None => crew_theme::theme().ink,
    };
    // Two leading spaces: a fence's rendered lines carry the card's indent,
    // and the marker column is what the refinement counts from.
    "  ".chars()
        .map(|c| plain(c, crew_theme::theme().ink, false))
        .chain(text.chars().map(|c| plain(c, fg, false)))
        .collect()
}

fn dimmed(l: &CardLine, at: usize) -> bool {
    let full = crate::chatink::token_fg(Token::Removed);
    let added = crate::chatink::token_fg(Token::Added);
    let fg = l[at].fg;
    fg != full && fg != added
}

/// The pair is found by the ink the renderer already gave each line — no
/// second parse of the source, and it works wherever the fence was rendered.
#[test]
fn a_paired_change_dims_what_the_two_lines_share() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        line("-let a = 1;", Some(Token::Removed)),
        line("+let a = 2;", Some(Token::Added)),
    ];
    refine_lines(&mut lines);
    // Column 2 is the marker, 3.. is the text: "let a = " is shared, the
    // digit is not.
    let digit = 2 + "-let a = ".len();
    assert!(!dimmed(&lines[0], digit), "the change was dimmed");
    assert!(lines[0][digit].bold, "the change is not marked");
    assert!(
        dimmed(&lines[0], 4),
        "the shared text stayed at full strength"
    );
    assert!(!dimmed(&lines[0], 2), "the marker was dimmed");
    assert!(!dimmed(&lines[1], digit) && lines[1][digit].bold);
}

/// Unequal runs have no honest correspondence, so nothing is marked.
#[test]
fn unequal_runs_are_left_alone() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        line("-a = 1;", Some(Token::Removed)),
        line("-b = 2;", Some(Token::Removed)),
        line("+a = 9;", Some(Token::Added)),
    ];
    let before: Vec<(u8, u8, u8)> = lines.iter().flat_map(|l| l.iter().map(|c| c.fg)).collect();
    refine_lines(&mut lines);
    let after: Vec<(u8, u8, u8)> = lines.iter().flat_map(|l| l.iter().map(|c| c.fg)).collect();
    assert_eq!(before, after);
}

/// Ordinary prose around a fence is not a diff and must not be touched.
#[test]
fn lines_that_are_not_diff_lines_are_untouched() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        line("here is a diff:", None),
        line("-a = 1;", Some(Token::Removed)),
        line("+a = 2;", Some(Token::Added)),
        line("that was it", None),
    ];
    let prose = lines[0].clone();
    let tail = lines[3].clone();
    refine_lines(&mut lines);
    let same = |a: &CardLine, b: &CardLine| {
        a.iter()
            .zip(b)
            .all(|(x, y)| x.fg == y.fg && x.bold == y.bold)
    };
    assert!(same(&lines[0], &prose) && same(&lines[3], &tail));
}

/// Several pairs in one hunk each refine on their own.
#[test]
fn every_pair_in_a_run_is_refined() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        line("-let a = 1;", Some(Token::Removed)),
        line("-let b = 1;", Some(Token::Removed)),
        line("+let a = 2;", Some(Token::Added)),
        line("+let b = 3;", Some(Token::Added)),
    ];
    refine_lines(&mut lines);
    for l in &lines {
        assert!(dimmed(l, 4), "a line in the run was not refined");
    }
}

/// An empty line, or one made only of its marker, must not panic the walk.
#[test]
fn degenerate_lines_are_survivable() {
    let _g = crate::app::theme_test_guard();
    let mut lines = vec![
        Vec::new(),
        line("-", Some(Token::Removed)),
        line("+", Some(Token::Added)),
        line("", Some(Token::Added)),
    ];
    refine_lines(&mut lines);
}
