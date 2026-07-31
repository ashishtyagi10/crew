use super::*;

#[test]
fn plain_rows_split_on_the_delimiter() {
    assert_eq!(
        parse("a,b\n1,2\n", ','),
        vec![
            vec!["a".to_string(), "b".into()],
            vec!["1".into(), "2".into()]
        ]
    );
}

#[test]
fn a_quoted_field_keeps_its_delimiter() {
    assert_eq!(
        parse("name,note\n\"Smith, A\",ok\n", ','),
        vec![
            vec!["name".to_string(), "note".into()],
            vec!["Smith, A".into(), "ok".into()],
        ]
    );
}

#[test]
fn a_doubled_quote_is_one_literal_quote() {
    assert_eq!(
        parse("a\n\"say \"\"hi\"\"\"\n", ','),
        vec![vec!["a".to_string()], vec!["say \"hi\"".to_string()],]
    );
}

#[test]
fn tabs_work_as_a_delimiter() {
    assert_eq!(
        parse("a\tb\n", '\t'),
        vec![vec!["a".to_string(), "b".into()]]
    );
}

#[test]
fn a_trailing_newline_does_not_make_an_empty_row() {
    assert_eq!(parse("a,b\n", ',').len(), 1);
}

#[test]
fn an_embedded_newline_inside_quotes_ends_the_row() {
    // Documented limitation, asserted so it is a decision and not a surprise:
    // a quoted field spanning lines splits. Rare in practice, and the
    // alternative is a streaming parser this pane does not need.
    assert_eq!(parse("\"two\nlines\"\n", ',').len(), 2);
}

#[test]
fn crlf_line_endings_do_not_leave_a_stray_carriage_return() {
    // Excel and most Windows/web CSV exports use CRLF. Without stripping it,
    // the last field of every row keeps a \r that reaches the renderer as
    // literal cell text.
    assert_eq!(
        parse("a,b\r\n1,2\r\n", ','),
        vec![
            vec!["a".to_string(), "b".into()],
            vec!["1".into(), "2".into()],
        ]
    );
}

#[test]
fn rendered_columns_align() {
    let ls = lines("name,n\nlong-value,1\nx,2\n", ',', 60);
    let texts: Vec<String> = ls.iter().map(|l| l.iter().map(|c| c.c).collect()).collect();
    assert!(texts.len() >= 3, "header, rule and rows: {texts:?}");
    let a = texts[2].find('1');
    let b = texts.last().unwrap().find('2');
    assert_eq!(a, b, "the second column starts at one column: {texts:?}");
}

#[test]
fn an_empty_file_renders_nothing_and_does_not_panic() {
    assert!(lines("", ',', 40).is_empty());
}
