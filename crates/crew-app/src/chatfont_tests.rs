use super::*;

#[test]
fn parse_extracts_the_font_arg() {
    assert_eq!(parse("/font random"), Some("random".to_string()));
    assert_eq!(parse("/font"), Some(String::new()));
    assert_eq!(parse("  /font 18  "), Some("18".to_string()));
}

#[test]
fn parse_rejects_foreign_text() {
    assert_eq!(parse("/fontx"), None);
    assert_eq!(parse("/ font"), None);
    assert_eq!(parse("hello /font"), None);
}
