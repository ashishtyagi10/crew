use super::*;

#[test]
fn a_logical_line_joins_the_rows_that_wrap() {
    let rows: Vec<Vec<char>> = ["ab", "cd", "ef", "gh"]
        .map(|r| r.chars().collect())
        .to_vec();
    // Rows 1 and 2 wrap onto the next: 1-2-3 is one line.
    let wraps = [false, true, true, false];
    let text =
        |(line, at, first): (Vec<char>, usize, usize)| (line.into_iter().collect(), at, first);
    let joined: (String, _, _) = text(logical_line(&rows, &wraps, 2, 1).unwrap());
    assert_eq!(joined, ("cdefgh".into(), 3, 1));
    let alone: (String, _, _) = text(logical_line(&rows, &wraps, 0, 0).unwrap());
    assert_eq!(alone, ("ab".into(), 0, 0));
    assert!(logical_line(&rows, &wraps, 4, 0).is_none());
}
